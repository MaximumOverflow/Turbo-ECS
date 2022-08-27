use std::sync::atomic::{AtomicUsize, Ordering};
use std::hash::{BuildHasherDefault, Hash};
use crate::data_structures::BitField;
use crate::components::Component;
use nohash_hasher::NoHashHasher;
use std::collections::HashMap;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use std::any::TypeId;

type Hasher = BuildHasherDefault<NoHashHasher<usize>>;
type IdMap = HashMap<TypeId, ComponentId, Hasher>;

lazy_static! {
	static ref COMPONENT_IDS: RwLock<IdMap> = RwLock::new(HashMap::default());
}

static mut NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// A `ComponentId` represents a globally unique identifier for a type implementing the [`Component`] trait.
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct ComponentId {
	value: usize,
}

impl ComponentId {
	/// Get the [ComponentId] of the type `T`
	pub fn of<T: 'static + Component>() -> ComponentId {
		let ids = COMPONENT_IDS.read();
		match ids.get(&TypeId::of::<T>()) {
			Some(id) => *id,
			None => {
				drop(ids);
				create_id::<T>()
			},
		}
	}

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

#[inline(never)]
fn create_id<T: 'static + Component>() -> ComponentId {
	unsafe {
		let mut ids = COMPONENT_IDS.write();
		let id = ComponentId { value: NEXT_ID.fetch_add(1, Ordering::Relaxed), };
		ids.insert(TypeId::of::<T>(), id);
		id
	}
}
