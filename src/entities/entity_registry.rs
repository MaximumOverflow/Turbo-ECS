use crate::entities::{assert_entity, ComponentQuery, Entity, EntityInstance};
use crate::archetypes::{
	Archetype, ArchetypeInstance, ArchetypeStore, ArchetypeTransition, ArchetypeTransitionKind, IterArchetype,
	IterArchetypeParallel,
};
use crate::components::{Component, ComponentSet, ComponentType};
use crate::data_structures::{BitField, Pool, RangeAllocator};
use std::ops::{DerefMut, Range};
use std::marker::PhantomData;
use std::iter::repeat_with;

/// A container for [Entities](crate::entities::Entity) and their associated [Components](crate::components::Component).
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

	/// Creates a single [entity](Entity) with no [components](Component) attached.
	pub fn create_entity(&mut self) -> Entity {
		self.create_entity_from_archetype(Archetype::default())
	}

	/// Creates a single [entity](Entity) belonging to the specified [archetype](Archetype).
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

	/// Creates a series of [entities](Entity) belonging to the specified [archetype](Archetype).  
	/// The new [entities](Entity) will be written into the provided slice.
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

	/// Destroys the provided [entities](Entity).  
	/// This function will panic if it encounters an invalid [entity](Entity).
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

	/// Gets a reference to a [component](Component) bound to a specific [entity](Entity).
	pub fn get_component<T: Component>(&self, entity: &Entity) -> Option<&T> {
		let instance = &self.instances[entity.index as usize];
		assert_entity(entity, instance);

		let archetype = self.archetype_store.get(instance.archetype as usize);
		let component = archetype.get_component::<T>(instance.slot as usize)?;
		unsafe { Some(&*(component as *const T)) }
	}

	/// Gets a mutable reference to a [component](Component) bound to a specific [entity](Entity).
	pub fn get_component_mut<T: Component>(&mut self, entity: &Entity) -> Option<&mut T> {
		let instance = &self.instances[entity.index as usize];
		assert_entity(entity, instance);

		let archetype = self.archetype_store.get_mut(instance.archetype as usize);
		let component = archetype.get_component_mut::<T>(instance.slot as usize)?;
		unsafe { Some(&mut *(component as *mut T)) }
	}

	/// Add a new [component](Component) to the specified [entity](Entity).  
	/// The function will return *false* if a [component](Component) of the same type is already present.
	pub fn add_component<T: Component>(&mut self, entity: &Entity, value: T) -> bool {
		let component = ComponentType::of::<T>();
		let kind = ArchetypeTransitionKind::Add;
		let transition = self.apply_archetype_transition(entity, component, kind);

		match transition {
			None => false,
			Some((_, (archetype, slot))) => unsafe {
				let dst = self.archetype_store.get_mut(archetype.index);
				std::ptr::write(dst.get_component_mut(slot).unwrap(), value);
				true
			},
		}
	}

	/// Remove a [component](Component) from the specified [entity](Entity).  
	/// The function will return *false* if the [component](Component) is not present.
	pub fn remove_component<T: Component>(&mut self, entity: &Entity) -> bool {
		let component = ComponentType::of::<T>();
		let kind = ArchetypeTransitionKind::Remove;
		let transition = self.apply_archetype_transition(entity, component, kind);

		match transition {
			None => false,
			Some(((archetype, slot), _)) => unsafe {
				let src = self.archetype_store.get_mut(archetype.index);
				std::ptr::drop_in_place(src.get_component_mut::<T>(slot).unwrap());
				true
			},
		}
	}

	/// Create a new filter for the currently existing [entities](Entity).
	///
	/// The filter can then be used to iterate over those [entities](Entity)
	/// or perform other kinds of operations.
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

	#[inline(never)]
	fn apply_archetype_transition(
		&mut self, entity: &Entity, component: ComponentType, kind: ArchetypeTransitionKind,
	) -> Option<((Archetype, usize), (Archetype, usize))> {
		let instance = &mut self.instances[entity.index as usize];
		assert_entity(entity, instance);

		let transition = self.archetype_store.get_archetype_transition(ArchetypeTransition {
			archetype: Archetype {
				index: instance.archetype as usize,
			},
			component,
			kind,
		});

		let (src, dst) = match transition {
			None => return None,
			Some((src, dst)) => (src, dst),
		};

		let src_slot = instance.slot as usize;
		instance.archetype = dst.id().index as u16;

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
		}

		Some(((src.id(), src_slot), (dst.id(), dst_slot)))
	}
}

/// It defines the set of [components](Component) an [entity](Entity) must or must not include.
pub struct EntityFilter<'l, I: 'static + ComponentSet, E: 'static + ComponentSet> {
	entity_store: &'l mut EntityRegistry,
	i_phantom: PhantomData<&'l I>,
	e_phantom: PhantomData<&'l E>,
}

/// It allows for iteration over a set of matching [entities](Entity) in an [EntityFilter].
pub trait EntityFilterForEach<I: 'static + ComponentSet, E: 'static + ComponentSet>
where
	ArchetypeInstance: IterArchetype<I>,
{
	/// Iterate all matching entities with the provided function.
	fn for_each(self, func: impl FnMut(<(I, E) as ComponentQuery>::Arguments));
}

/// It allows for parallel iteration over a set of matching [entities](Entity) in an [EntityFilter].
pub trait EntityFilterParallelForEach<I: 'static + ComponentSet, E: 'static + ComponentSet>
where
	ArchetypeInstance: IterArchetypeParallel<I>,
{
	/// Iterate all matching entities in parallel with the provided function.
	fn par_for_each(self, func: (impl Fn(<(I, E) as ComponentQuery>::Arguments) + Send + Sync));
}

impl<'l, I: 'static + ComponentSet, E: 'static + ComponentSet> EntityFilter<'l, I, E> {
	/// It specifies which [components](Component) an [entity](Entity) must include to be picked up by the [EntityFilter].  
	/// This function creates a new [EntityFilter] each time it's invoked, so it should ideally only be called once
	/// with all the desired [component](Component) types.
	pub fn include<TI: 'static + ComponentSet>(self) -> EntityFilter<'l, TI, E> {
		EntityFilter {
			entity_store: self.entity_store,
			i_phantom: PhantomData::default(),
			e_phantom: PhantomData::default(),
		}
	}

	/// It specifies which [components](Component) an [entity](Entity) must not include to be picked up by the [EntityFilter].  
	/// This function creates a new [EntityFilter] each time it's invoked, so it should ideally only be called once
	/// with all the desired [component](Component) types.
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
	ArchetypeInstance: IterArchetype<I>,
{
	fn for_each(self, mut func: impl FnMut(<(I, E) as ComponentQuery>::Arguments)) {
		let query = <(I, E)>::get_query();
		for archetype in self.entity_store.archetype_store.query(query) {
			IterArchetype::for_each_mut(archetype, &mut func);
		}
	}
}

impl<I: 'static + ComponentSet, E: 'static + ComponentSet> EntityFilterParallelForEach<I, E> for EntityFilter<'_, I, E>
where
	ArchetypeInstance: IterArchetypeParallel<I>,
{
	fn par_for_each(self, func: (impl Fn(<(I, E) as ComponentQuery>::Arguments) + Send + Sync)) {
		let query = <(I, E)>::get_query();

		self.entity_store
			.archetype_store
			.query(query)
			.for_each(|archetype| IterArchetypeParallel::for_each_mut(archetype, &func));
	}
}
