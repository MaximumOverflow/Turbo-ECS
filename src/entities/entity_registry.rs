use crate::archetypes::{Archetype, ArchetypeInstance, ArchetypeStore, IterateArchetype, IterateArchetypeParallel};
use crate::entities::{assert_entity, ComponentQuery, Entity, EntityInstance};
use crate::components::{Component, ComponentSet, ComponentType};
use crate::data_structures::{BitField, Pool, RangeAllocator};
use crate::components::component_id::HasComponentId;
use std::ops::{DerefMut, Range};
use std::marker::PhantomData;
use std::iter::repeat_with;

pub struct EntityRegistry {
	allocator: RangeAllocator,
	instances: Vec<EntityInstance>,
	pub(crate) archetype_store: ArchetypeStore,

	bitfield: BitField,
	usize_vec_pool: Pool<Vec<usize>>,
	range_vec_pool: Pool<Vec<Range<usize>>>,
}

impl EntityRegistry {
	pub(crate) fn new() -> Self {
		Self {
			instances: vec![],
			allocator: RangeAllocator::new(),
			archetype_store: ArchetypeStore::new(),

			bitfield: BitField::new(),
			usize_vec_pool: Pool::default(),
			range_vec_pool: Pool::default(),
		}
	}

	/// Creates a single [`entity`](Entity) with no [`components`](Component) attached.
	pub fn create_entity(&mut self) -> Entity {
		self.create_entity_from_archetype(Archetype::default())
	}

	/// Creates a single [`entity`](Entity) belonging to the specified [`archetype`](Archetype).
	/// # Arguments
	/// * `archetype` - The [`archetype`](Archetype) from which to construct the [`entity`](Entity) instances.
	#[inline(never)]
	pub fn create_entity_from_archetype(&mut self, archetype: Archetype) -> Entity {
		let index = match self.allocator.try_allocate(1) {
			Ok(index) => index.start,
			Err(_) => {
				let capacity = usize::max(1, self.allocator.capacity());
				self.reserve_entity_space(capacity);
				self.allocator.allocate(1).start
			},
		};

		let instance = &mut self.instances[index];
		let mut slot_ranges = self.range_vec_pool.take_one();

		let archetype_instance = self.archetype_store.get_mut(archetype.index as usize);
		archetype_instance.take_slots(1, &mut slot_ranges);

		instance.slot = slot_ranges[0].start as u32;
		instance.archetype = archetype.index as u16;

		Entity {
			index: index as u32,
			version: instance.version,
		}
	}

	/// Creates a series of [`entities`](Entity) belonging to the specified [`archetype`](Archetype).
	/// # Arguments
	/// * `archetype` - The [`archetype`](Archetype) from which to construct the [`entity`](Entity) instances.
	/// * `entities` - The slice in which to output the [`entity`](Entity) instances.
	#[inline(never)]
	pub fn create_entities_from_archetype(&mut self, archetype: Archetype, entities: &mut [Entity]) {
		let count = entities.len();
		let mut slot_ranges = self.range_vec_pool.take_one();
		let mut instance_ranges = self.range_vec_pool.take_one();

		match self.allocator.try_allocate_fragmented(count, &mut instance_ranges) {
			Ok(_) => {},
			Err(needed) => {
				let target_capacity = usize::max(self.allocator.capacity() * 2, needed);
				self.reserve_entity_space(target_capacity - self.allocator.capacity());
				self.allocator.allocate_fragmented(count, &mut instance_ranges);
			},
		}

		let archetype_instance = self.archetype_store.get_mut(archetype.index);
		archetype_instance.take_slots(count, &mut slot_ranges);

		let entity_iter = 0..count;
		let slot_iter = slot_ranges.iter().flat_map(|i| i.clone());
		let instance_iter = instance_ranges.iter().flat_map(|i| i.clone());

		let a = archetype.index as u16;
		for ((i, e), s) in instance_iter.zip(entity_iter).zip(slot_iter) {
			let instance = &mut self.instances[i];
			let entity = &mut entities[e];

			instance.archetype = a;
			instance.slot = s as u32;

			entity.index = i as u32;
			entity.version = instance.version;
		}
	}

	/// Destroys the provided [`entities`](Entity).
	/// This function will panic if it encounters an invalid [`entity`](Entity).
	#[inline(never)]
	pub fn destroy_entities(&mut self, entities: &[Entity]) {
		unsafe {
			self.bitfield.clear();
			let mut slots = self.usize_vec_pool.take_one();
			let slots = slots.deref_mut();

			slots.set_len(0);
			if entities.len() > slots.capacity() {
				slots.reserve(entities.len() - slots.capacity())
			}

			let mut last_archetype = 0;
			let archetypes = &mut self.archetype_store;

			for entity in entities {
				let index = entity.index as usize;
				let instance = &mut self.instances[index];

				assert_entity(entity, instance);
				self.bitfield.set_inlined_unchecked(index, true);

				let archetype = instance.archetype;
				if (archetype != last_archetype) & !slots.is_empty() {
					archetypes.get_mut(last_archetype as usize).return_slots(slots);
					slots.set_len(0);
				}

				last_archetype = archetype;
				slots.push(instance.slot as usize);
			}

			if !slots.is_empty() {
				archetypes.get_mut(last_archetype as usize).return_slots(slots);
			}

			for range in self.bitfield.iter_ranges() {
				for i in range.clone() {
					self.instances[i].version += 1;
				}
				self.allocator.free(range);
			}
		}
	}

	/// Gets a reference to a [`component`](Component) bound to a specific [`entity`](Entity).
	pub fn get_component<T: 'static + Component + HasComponentId>(&self, entity: &Entity) -> Option<&T> {
		let instance = &self.instances[entity.index as usize];
		assert_entity(entity, instance);

		let archetype = self.archetype_store.get(instance.archetype as usize);
		let component = archetype.get_component::<T>(instance.slot as usize)?;
		unsafe { Some(&*(component as *const T)) }
	}

	/// Gets a mutable reference to a [`component`](Component) bound to a specific [`entity`](Entity).
	pub fn get_component_mut<T: 'static + Component + HasComponentId>(&mut self, entity: &Entity) -> Option<&mut T> {
		let instance = &self.instances[entity.index as usize];
		assert_entity(entity, instance);

		let archetype = self.archetype_store.get_mut(instance.archetype as usize);
		let component = archetype.get_component_mut::<T>(instance.slot as usize)?;
		unsafe { Some(&mut *(component as *mut T)) }
	}

	pub fn add_component_data<T: 'static + Component + HasComponentId>(&mut self, entity: &Entity, value: T) -> bool {
		let instance = &mut self.instances[entity.index as usize];
		assert_entity(entity, instance);

		let (src, dst) = {
			let dst_archetype_id = {
				let src_archetype = self.archetype_store.get(instance.archetype as usize);
				if src_archetype.component_bitfield().get(T::component_id().value()) {
					return false;
				}

				let mut components = Vec::from(src_archetype.components());

				components.push(ComponentType::of::<T>());
				self.archetype_store.create_archetype(&components)
			};

			// SAFETY: Always safe.
			// The two ArchetypeInstance references cannot overlap.
			unsafe {
				let dst_archetype = self.archetype_store.get_mut(dst_archetype_id.index);
				let dst_archetype = &mut *(dst_archetype as *mut ArchetypeInstance);

				let src_archetype = self.archetype_store.get_mut(instance.archetype as usize);
				let src_archetype = &mut *(src_archetype as *mut ArchetypeInstance);

				instance.archetype = dst_archetype_id.index as u16;
				(src_archetype, dst_archetype)
			}
		};

		let src_slot = instance.slot as usize;

		let dst_slot = {
			let mut slots = self.range_vec_pool.take_one();
			dst.take_slots_no_init(1, &mut slots);

			let slot = slots[0].start;
			instance.slot = slot as u32;
			slot
		};

		// SAFETY: Always safe.
		// Ownership of all components is transferred to the destination archetype, so we don't call drop on them.
		// The component data in the source archetype can be safely overwritten by subsequent allocations.
		// All components in the destination archetype will have already been dropped by a previous deallocation,
		// so they can be safely overwritten too.
		unsafe {
			src.copy_components(dst, src_slot, dst_slot);
			src.return_slots_no_drop(std::slice::from_ref(&src_slot));
			std::ptr::write(dst.get_component_mut(dst_slot).unwrap(), value);
		}

		true
	}

	#[inline(always)]
	pub fn filter(&mut self) -> EntityFilter<(), ()> {
		EntityFilter {
			entity_store: self,
			i_phantom: PhantomData::default(),
			e_phantom: PhantomData::default(),
		}
	}

	fn reserve_entity_space(&mut self, size: usize) {
		self.allocator.reserve(size);
		self.bitfield.reserve(size);
		self.instances.extend(repeat_with(Default::default).take(size));
	}
}

pub struct EntityFilter<'l, I: 'static + ComponentSet, E: 'static + ComponentSet> {
	entity_store: &'l mut EntityRegistry,
	i_phantom: PhantomData<&'l I>,
	e_phantom: PhantomData<&'l E>,
}

pub trait EntityFilterForEach<I: 'static + ComponentSet, E: 'static + ComponentSet>
where
	ArchetypeInstance: IterateArchetype<I>,
{
	fn for_each(self, func: impl FnMut(<(I, E) as ComponentQuery>::Arguments));
}

pub trait EntityFilterParallelForEach<I: 'static + ComponentSet, E: 'static + ComponentSet>
where
	ArchetypeInstance: IterateArchetypeParallel<I>,
{
	fn par_for_each(self, func: (impl Fn(<(I, E) as ComponentQuery>::Arguments) + Send + Sync));
}

impl<'l, I: 'static + ComponentSet, E: 'static + ComponentSet> EntityFilter<'l, I, E> {
	pub fn include<TI: 'static + ComponentSet>(self) -> EntityFilter<'l, TI, E> {
		EntityFilter {
			entity_store: self.entity_store,
			i_phantom: PhantomData::default(),
			e_phantom: PhantomData::default(),
		}
	}

	pub fn exclude<TE: 'static + ComponentSet>(self) -> EntityFilter<'l, I, TE> {
		EntityFilter {
			entity_store: self.entity_store,
			i_phantom: PhantomData::default(),
			e_phantom: PhantomData::default(),
		}
	}
}

impl<I: 'static + ComponentSet, E: 'static + ComponentSet> EntityFilterForEach<I, E> for EntityFilter<'_, I, E>
where
	ArchetypeInstance: IterateArchetype<I>,
{
	fn for_each(self, mut func: impl FnMut(<(I, E) as ComponentQuery>::Arguments)) {
		let query = <(I, E)>::get_query();
		for archetype in self.entity_store.archetype_store.query(query) {
			IterateArchetype::for_each_mut(archetype, &mut func);
		}
	}
}

impl<I: 'static + ComponentSet, E: 'static + ComponentSet> EntityFilterParallelForEach<I, E> for EntityFilter<'_, I, E>
where
	ArchetypeInstance: IterateArchetypeParallel<I>,
{
	fn par_for_each(self, func: (impl Fn(<(I, E) as ComponentQuery>::Arguments) + Send + Sync)) {
		let query = <(I, E)>::get_query();

		self.entity_store
			.archetype_store
			.query(query)
			.for_each(|archetype| IterateArchetypeParallel::for_each_mut(archetype, &func));
	}
}
