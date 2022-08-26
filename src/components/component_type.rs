use crate::data_structures::{AnyVec, BitField};
use crate::components::ComponentId;
use std::hash::{Hash, Hasher};

/// A runtime representation of a type implementing the [`Component`] trait.
#[derive(Clone)]
pub struct ComponentType {
	id: ComponentId,
	make_vec: fn() -> AnyVec,
}

impl ComponentType {
	/// Returns the [`ComponentType`] of T.
	pub fn of<T: 'static + Copy + Default + Component>() -> Self {
		Self {
			id: ComponentId::of::<T>(),
			make_vec: AnyVec::new::<T>,
		}
	}

	pub const fn id(&self) -> ComponentId {
		self.id
	}

	pub fn make_vec(&self) -> AnyVec {
		(self.make_vec)()
	}
}

impl Eq for ComponentType {}

impl PartialEq<Self> for ComponentType {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}

impl Hash for ComponentType {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id.hash(state)
	}
}

pub trait Component
where
	Self: Copy + Default,
{
}

/// This trait should only be implemented by #\[derive([`Component`])] for use by IterArchetype.
/// It provides a unified way to access a component's id and type through its base type and all derived ref types.
pub trait ComponentTypeInfo {
	type ComponentType: ComponentTypeInfo;
	fn component_id() -> ComponentId;
}

/// This trait should only be implemented by #\[derive([`Component`])] for use by IterArchetype.
/// #\[derive([`Component`])] will generate the following trait implementations:
/// - ConvertFrom<T> for T
/// - ConvertFrom<*const T> for T
/// - ConvertFrom<*const T> for &T
/// - ConvertFrom<*mut T> for T
/// - ConvertFrom<*mut T> for &T
/// - ConvertFrom<*mut T> for &mut T
pub trait ComponentFrom<T> {
	/// # Safety
	/// This function should only be implemented by #\[derive(Component)] for use by IterArchetype.
	/// IterArchetype's implementation guarantees rust's aliasing rules are maintained.
	unsafe fn convert(value: T) -> Self;
}

impl<T: ComponentTypeInfo> ComponentTypeInfo for &T {
	type ComponentType = T::ComponentType;
	fn component_id() -> ComponentId {
		Self::ComponentType::component_id()
	}
}

impl<T: ComponentTypeInfo> ComponentTypeInfo for &mut T {
	type ComponentType = T::ComponentType;
	fn component_id() -> ComponentId {
		Self::ComponentType::component_id()
	}
}

impl From<&[ComponentType]> for BitField {
	fn from(ids: &[ComponentType]) -> Self {
		let mut bitfield = BitField::new();
		for ty in ids {
			bitfield.set(ty.id().value(), true);
		}

		bitfield
	}
}
