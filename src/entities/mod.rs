//! [Entities](Entity) represent the individual "things" in your game or application.
//!
//! An [Entity] doesn't store any data and has no associated behaviour;  
//! instead, it identifies which pieces of data ([Components](crate::components::Component)) belong together.
//!
//! TODO

mod entity_query;
mod entity_registry;
mod entity_instance;

pub use entity_query::*;
pub use entity_registry::*;
pub use entity_instance::*;
