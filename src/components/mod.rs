//! [Components](Component) are pieces of data associated with one or more [entities](crate::entities::Entity).
//!
//! Turbo ECS isn't particularly picky as to what components can contain or how they're represented:  
//! all a type needs to do to be considered a valid [component](Component) is to derive the [`Component`] trait.
//!
//! [Components](Component) are stored in contiguous memory chunks managed by an [archetype](crate::archetypes::Archetype).
//!
//! All [Component] types will have a unique [ComponentId] automatically assigned at runtime.  
//! Developers shouldn't rely on those [component ids](ComponentId), as they are not stable between program re-runs.
//!
//! [Components](Component) can be dynamically added and removed from any [entity](crate::entities::Entity).  
//! It should be noted, though, that this is a costly operation that requires all other [components](Component)
//! to be moved from one [archetype](crate::archetypes::Archetype) to another; this may also increase memory fragmentation.  
//! Due to these reasons, structural changes should be kept to a minimum.

pub mod component_id;
mod component_type;
mod component_set;

pub use component_set::*;
pub use component_type::*;
pub use turbo_ecs_derive::Component;
pub(crate) use component_id::{ComponentId};
