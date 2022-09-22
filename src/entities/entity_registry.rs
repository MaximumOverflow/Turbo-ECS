use crate::archetypes::{
	Archetype, ArchetypeInstance, ArchetypeStore, ArchetypeTransition, ArchetypeTransitionKind, IterArchetype,
	IterArchetypeParallel,
};
use crate::components::{Component, ComponentSet, ComponentType};
use crate::entities::{ComponentQuery, Entity, EntityInstance};
use crate::data_structures::{BitField, Pool};
use std::sync::atomic::{AtomicU32, Ordering};
use std::marker::PhantomData;
use std::alloc::Layout;
use std::ops::Range;

static mut NEXT_ID: AtomicU32 = AtomicU32::new(1);

/// A container for [Entities](crate::entities::Entity) and their associated [Components](crate::components::Component).
pub struct EntityRegistry {
	id: u32,
	capacity: usize,
	instance_buffers: Vec<Box<[EntityInstance]>>,
	available_instances: Vec<*mut EntityInstance>,

	pub(crate) archetype_store: ArchetypeStore,

	bitfield: BitField,
	usize_vec_pool: Pool<Vec<usize>>,
	range_vec_pool: Pool<Vec<Range<usize>>>,
}

impl EntityRegistry {
	pub(crate) fn new() -> Self {
		Self {
			id: unsafe { NEXT_ID.fetch_and(1, Ordering::Relaxed) },

			capacity: 0,
			instance_buffers: vec![],
			available_instances: vec![],
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
		let instance = match self.available_instances.pop() {
			None => unsafe {
				self.new_instance_buffer(usize::max(16, self.capacity));
				&mut *self.available_instances.pop().unwrap()
			},

			Some(instance) => unsafe { &mut *instance },
		};

		let mut slot_ranges = self.range_vec_pool.take_one();

		let archetype_instance = self.archetype_store.get_mut(archetype.index as usize);
		archetype_instance.take_slots(1, &mut slot_ranges);

		instance.slot = slot_ranges[0].start;
		instance.archetype = archetype.index;

		Entity {
			instance,
			registry_id: self.id,
			version: instance.version,
		}
	}

	/// Creates a series of [entities](Entity) belonging to the specified [archetype](Archetype).  
	/// The new [entities](Entity) will be written into the provided slice.
	#[inline(never)]
	pub fn create_entities_from_archetype(
		&mut self, archetype: Archetype, count: usize,
	) -> impl Iterator<Item = Entity> + '_ {
		if self.available_instances.len() < count {
			let required = count - self.available_instances.len();
			self.new_instance_buffer(usize::max(required, self.capacity));
		}

		let context_id = self.id;
		let archetype_id = archetype.index;

		let end = self.available_instances.len();
		let start = self.available_instances.len() - count;
		let instances = &mut self.available_instances.as_mut_slice()[start..];

		let mut slots = vec![];
		let archetype = self.archetype_store.get_mut(archetype_id);

		archetype.take_slots(count, &mut slots);
		let archetype_entities = archetype.entities_mut();

		unsafe {
			let mut slots = slots.iter().cloned().flatten();

			for i in 0..count {
				let next = slots.next();
				debug_assert_ne!(next, None);

				let slot = next.unwrap_unchecked();
				let instance = &mut *instances[i];

				instance.slot = slot;
				instance.archetype = archetype_id;

				let entity = Entity {
					instance,
					registry_id: context_id,
					version: instance.version,
				};

				archetype_entities[slot] = entity;
			}
		}

		self.available_instances.drain(start..end);

		slots.into_iter().flatten().map(|i| archetype_entities[i].clone())
	}

	/// Destroys the provided [entities](Entity).  
	/// This function will panic if it encounters an invalid [entity](Entity).
	#[inline(never)]
	pub fn destroy_entities(&mut self, entities: &[Entity]) {
		unsafe {
			self.bitfield.clear();
			let mut slots = self.usize_vec_pool.take_one();

			slots.clear();
			if entities.len() > slots.capacity() {
				let reserve = entities.len() - slots.capacity();
				slots.reserve(reserve)
			}

			let mut last_archetype = 0;
			let archetypes = &mut self.archetype_store;

			for entity in entities {
				let mut entity = entity.clone();
				let instance = entity.get_instance_mut(self.id);

				let archetype = instance.archetype;
				if (archetype != last_archetype) & !slots.is_empty() {
					archetypes.get_mut(last_archetype).return_slots(&slots);
					self.bitfield.clear();
					slots.clear()
				}

				if !self.bitfield.get_inlined_unchecked(instance.slot) {
					instance.version += 1;
					last_archetype = archetype;
					slots.push(instance.slot as usize);
					self.bitfield.set_inlined_unchecked(instance.slot, true);
				}
			}

			if !slots.is_empty() {
				archetypes.get_mut(last_archetype as usize).return_slots(&slots);
			}
		}
	}

	/// Gets a reference to a [component](Component) bound to a specific [entity](Entity).
	pub fn get_component<T: Component>(&self, entity: &Entity) -> Option<&T> {
		let instance = entity.get_instance(self.id);
		let archetype = self.archetype_store.get(instance.archetype as usize);
		let component = archetype.get_component::<T>(instance.slot as usize)?;
		unsafe { Some(&*(component as *const T)) }
	}

	/// Gets a mutable reference to a [component](Component) bound to a specific [entity](Entity).
	pub fn get_component_mut<T: Component>(&mut self, entity: &Entity) -> Option<&mut T> {
		let instance = entity.get_instance(self.id);
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

	fn new_instance_buffer(&mut self, size: usize) -> &mut [EntityInstance] {
		unsafe {
			let ptr = std::alloc::alloc(Layout::array::<EntityInstance>(size).unwrap()) as *mut EntityInstance;
			let buffer = std::slice::from_raw_parts_mut(ptr, size);
			let instances = Box::from_raw(buffer);

			self.capacity += size;
			self.bitfield.reserve(size);
			self.instance_buffers.push(instances);
			buffer.fill_with(EntityInstance::default);

			for i in 0..size {
				self.available_instances.push(ptr.add(i));
			}

			buffer
		}
	}

	#[inline(never)]
	fn apply_archetype_transition(
		&mut self, entity: &Entity, component: ComponentType, kind: ArchetypeTransitionKind,
	) -> Option<((Archetype, usize), (Archetype, usize))> {
		let mut entity = entity.clone();
		let instance = entity.get_instance_mut(self.id);

		let transition = self.archetype_store.get_archetype_transition(ArchetypeTransition {
			archetype: Archetype {
				index: instance.archetype,
			},
			component,
			kind,
		});

		let (src, dst) = match transition {
			None => return None,
			Some((src, dst)) => (src, dst),
		};

		let src_slot = instance.slot as usize;
		instance.archetype = dst.id().index;

		let dst_slot = {
			let mut slots = self.range_vec_pool.take_one();
			dst.take_slots_no_init(1, &mut slots);

			let slot = slots[0].start;
			instance.slot = slot;
			slot
		};

		// SAFETY: Always safe.
		// Ownership of all components is transferred to the destination archetype, so we don't call drop on them.
		// The component data in the source archetype can be safely overwritten by subsequent allocations.
		// All components in the destination archetype will have already been dropped by a previous deallocation,
		// so they can be safely overwritten too.
		unsafe {
			src.copy_components(dst, src_slot, dst_slot);
			src.return_slot_no_drop(src_slot);
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

	/// Iterate all matching entities with the provided function.
	fn entities_for_each(self, func: impl FnMut(Entity, <(I, E) as ComponentQuery>::Arguments));
}

/// It allows for parallel iteration over a set of matching [entities](Entity) in an [EntityFilter].
pub trait EntityFilterParallelForEach<I: 'static + ComponentSet, E: 'static + ComponentSet>
where
	ArchetypeInstance: IterArchetypeParallel<I>,
{
	/// Iterate all matching entities in parallel with the provided function.
	fn par_for_each(self, func: (impl Fn(<(I, E) as ComponentQuery>::Arguments) + Send + Sync));

	/// Iterate all matching entities in parallel with the provided function.
	fn par_entities_for_each(self, func: (impl Fn(Entity, <(I, E) as ComponentQuery>::Arguments) + Send + Sync));
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
			IterArchetype::for_each(archetype, &mut func);
		}
	}

	fn entities_for_each(self, mut func: impl FnMut(Entity, <(I, E) as ComponentQuery>::Arguments)) {
		let query = <(I, E)>::get_query();
		for archetype in self.entity_store.archetype_store.query(query) {
			IterArchetype::entities_for_each(archetype, &mut func);
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
			.for_each(|archetype| IterArchetypeParallel::for_each(archetype, &func));
	}

	fn par_entities_for_each(self, func: (impl Fn(Entity, <(I, E) as ComponentQuery>::Arguments) + Send + Sync)) {
		let query = <(I, E)>::get_query();

		self.entity_store
			.archetype_store
			.query(query)
			.for_each(|archetype| IterArchetypeParallel::entities_for_each(archetype, &func));
	}
}
