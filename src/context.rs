use crate::systems::{System, SystemStore};
use crate::components::ComponentType;
use crate::entities::EntityStore;
use crate::archetypes::Archetype;
use std::ops::{Deref, DerefMut};

/// A container for [`entities`](Entity) and systems.
///
/// All [`entities`](Entity) can be referenced using a unique identifier of type [`Entity`]
/// and can contain an arbitrary selection of [`components`](Component) based on their [`archetype`](Archetype).
pub struct EcsContext {
	entity_store: EntityStore,
	system_store: SystemStore,
}

impl EcsContext {
	pub fn new() -> Self {
		Self {
			entity_store: EntityStore::new(),
			system_store: SystemStore::new(),
		}
	}

	/// Creates an [`archetype`](Archetype) containing the specified [`components`](Component).
	pub fn create_archetype(&mut self, components: &[ComponentType]) -> Archetype {
		self.entity_store.archetype_store.create_archetype(components)
	}

	/// Creates an [`archetype`](Archetype) containing the specified [`components`](Component) with the specified capacity.
	pub fn create_archetype_with_capacity(
		&mut self, components: &[ComponentType], min_capacity: usize,
	) -> Archetype {
		self.entity_store.archetype_store.create_archetype_with_capacity(components, min_capacity)
	}

	/// Add a new [system](System) to the [EcsContext].
	pub fn register_system<T: 'static + System>(&mut self, system: T) {
		self.system_store.add_system(system);
	}

	/// Initialize all [systems](System)
	/// Must be called before any system can be run.
	pub fn setup_systems(&mut self) {
		self.system_store.setup_systems(&mut self.entity_store.archetype_store);
	}

	/// Execute all [systems](System).
	pub fn run_systems(&mut self) {
		self.system_store.run_systems(&mut self.entity_store);
	}
}

impl Default for EcsContext {
	fn default() -> Self {
		Self::new()
	}
}

impl Deref for EcsContext {
	type Target = EntityStore;

	#[inline(always)]
	fn deref(&self) -> &Self::Target {
		&self.entity_store
	}
}

impl DerefMut for EcsContext {
	#[inline(always)]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.entity_store
	}
}
