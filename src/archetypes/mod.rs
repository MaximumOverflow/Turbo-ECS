//! [Archetypes](Archetype) are a set of [components](crate::components::Component)
//! tied to one or more [entities](crate::entities::Entity).
//!
//! TODO

mod archetype_macros;
mod archetype_instance;
mod archetype_registry;

pub use archetype_macros::*;
pub use archetype_instance::Archetype;

pub(crate) use archetype_instance::*;
pub(crate) use archetype_registry::*;
