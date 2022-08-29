use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::atomic::Ordering::{Acquire, Relaxed};
use crate::data_structures::BitField;
use crate::components::Component;
use std::hash::Hash;

pub static mut NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// A `ComponentId` represents a globally unique identifier for a type implementing the [`Component`] trait.
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct ComponentId {
	value: usize,
}

pub trait HasComponentId where Self: Component {
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

#[inline(always)]
pub unsafe fn get_component_id(lock: &mut AtomicBool, value: &mut AtomicUsize) -> ComponentId {
	let index = value.load(Relaxed);
	if index != 0 {
		ComponentId { value: index }
	}
	else {
		get_next_id(lock, value)
	}
}

#[inline(never)]
unsafe fn get_next_id(lock: &mut AtomicBool, value: &mut AtomicUsize) -> ComponentId {
	loop {
		if lock.compare_exchange(false, true, Acquire, Relaxed).is_ok() {
			break;
		}
	}

	if value.load(Relaxed) != 0 {
		ComponentId { value: value.load(Ordering::Relaxed) }
	}
	else {
		let next = ComponentId { value: NEXT_ID.fetch_add(1, Relaxed) };
		value.store(next.value, Relaxed);
		next
	}
}