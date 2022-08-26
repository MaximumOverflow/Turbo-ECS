pub mod data_structures;
pub mod components;
pub mod archetypes;
pub mod entities;
pub mod systems;
mod context;

pub mod prelude {
	pub use crate::systems::*;
	pub use crate::components::*;
	pub use crate::context::EcsContext;
	pub use crate::entities::{Entity, EntityQuery, EntityStore, QueryBuilder};
}

#[cfg(test)]
mod tests;
