pub mod data_structures;
pub mod components;
pub mod entities;
pub mod systems;
mod archetypes;
mod context;

pub use lazy_static::lazy_static;

pub mod prelude {
	pub use crate::systems::*;
	pub use crate::components::*;
	pub use crate::context::EcsContext;
	pub use crate::archetypes::{Archetype, ArchetypeStore};
	pub use crate::entities::{
		Entity, EntityQuery, EntityStore, QueryBuilder, EntityFilterForEach,
		EntityFilterParallelForEach,
	};
}

#[cfg(test)]
mod tests;
