#![warn(missing_docs)]

//! Turbo ECS is a high performance Entity-Component-System library for Rust game projects.
//! # Getting started
//! TODO
//!
//! ## Components
//! TODO
//!
//! For more information, please refer to [Components](crate::components) and [Archetypes](crate::archetypes).
//!
//! ## Entities
//! TODO
//!
//! For more information, please refer to [Entities](crate::entities).
//!
//! ## Systems
//! TODO
//!
//! For more information, please refer to [Systems](crate::systems).
//!
//! ## Queries
//! TODO
//!
//! For more information, please refer to [Entities](crate::entities) and [Archetypes](crate::archetypes).

pub mod data_structures;
pub mod components;
pub mod entities;
pub mod systems;
pub mod archetypes;
mod context;

pub use lazy_static::lazy_static;

pub mod prelude {
	//! All essential types and traits used by Turbo ECS
	pub use crate::systems::{System};
	pub use crate::context::EcsContext;
	pub use crate::archetypes::Archetype;
	pub use crate::components::{Component};
	pub use crate::entities::{
		Entity, EntityQuery, EntityRegistry, QueryBuilder, EntityFilterForEach, EntityFilterParallelForEach,
	};
}

#[cfg(test)]
mod tests;
