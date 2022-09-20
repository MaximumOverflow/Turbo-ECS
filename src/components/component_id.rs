//! A  unique runtime identifier tied to a [Component] type.
//!
//! Developers shouldn't rely on [component ids](ComponentId), as they are not stable between program re-runs.
//! [Component ids](ComponentId) are generally used for populating the various
//! [bitfields](crate::data_structures::BitField) used in
//! [entity queries](crate::entities::EntityQuery).

use std::sync::atomic::Ordering::Relaxed;
use crate::data_structures::BitField;
use std::sync::atomic::AtomicUsize;
use crate::components::Component;
use std::hash::Hash;

static mut NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// A globally unique identifier for a type implementing the [`Component`] trait.
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct ComponentId {
	value: usize,
}

impl ComponentId {
	/// Get the [ComponentId] of the type `T`.
	#[inline(always)]
	pub fn of<T: Component>() -> ComponentId {
		T::component_id()
	}

	#[inline(always)]
	pub(crate) const fn value(&self) -> usize {
		self.value
	}
}

impl From<&[ComponentId]> for BitField {
	fn from(ids: &[ComponentId]) -> Self {
		let mut bitfield = BitField::new();
		for id in ids {
			bitfield.set(id.value(), true);
		}

		bitfield
	}
}

/// Generates a new [ComponentId]. **Should not be called from user code.**
///
/// # Safety
/// Always safe when called from library code for newly instantiated [components](Component).  
/// To be called from code generated from #[derive([Component])].
pub unsafe fn get_next() -> ComponentId {
	let value = NEXT_ID.fetch_add(1, Relaxed);
	debug_assert!(
		value <= u32::MAX as usize,
		"This is an insane number of components. Please seek help."
	);
	ComponentId { value }
}
