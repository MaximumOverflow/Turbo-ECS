//! [Systems](System) provide the logic for modifying the state of [Entities](crate::entities::Entity)
//! and their associated [Components](crate::components::Component).
//!
//! A [System] must be manually added to an [EcsContext](crate::context::EcsContext)
//! for it to become active during the execution of the program.

mod system;
mod system_registry;

pub use system::*;
pub(crate) use system_registry::*;
