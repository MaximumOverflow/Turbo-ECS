use std::sync::atomic::Ordering::Relaxed;
use crate::data_structures::BitField;
use std::sync::atomic::AtomicUsize;
use crate::components::Component;
use std::hash::Hash;

pub static mut NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// A `ComponentId` represents a globally unique identifier for a type implementing the [`Component`] trait.
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct ComponentId {
	value: usize,
}

pub trait HasComponentId
where
	Self: Component,
{
	fn component_id() -> ComponentId;
}

impl ComponentId {
	/// Get the [ComponentId] of the type `T`
	#[inline(always)]
	pub fn of<T: 'static + Component + HasComponentId>() -> ComponentId {
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

/// # Safety
/// To be called from code generated from #[derive([Component])].
/// Should not be called from user code.
pub unsafe fn get_next() -> ComponentId {
	ComponentId {
		value: NEXT_ID.fetch_add(1, Relaxed),
	}
}
