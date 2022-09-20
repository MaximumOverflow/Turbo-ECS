use crate::data_structures::{AnyBuffer, BitField};
use crate::components::ComponentId;
use std::hash::{Hash, Hasher};
use std::any::TypeId;

/// A piece of data associated with an Entity.
pub trait Component
where
	Self: 'static + Default,
{
	/// Retrieves the [Component] type's unique runtime identifier.
	fn component_id() -> ComponentId;
}

/// A runtime representation of a type implementing the [`Component`] trait.
#[derive(Clone)]
pub struct ComponentType {
	id: ComponentId,
	type_id: TypeId,
	make_vec: fn() -> AnyBuffer,
}

impl ComponentType {
	/// Retrieves the [ComponentType] of `T`
	pub fn of<T: Component>() -> Self {
		Self {
			id: ComponentId::of::<T>(),
			type_id: TypeId::of::<T>(),
			make_vec: AnyBuffer::new_default::<T>,
		}
	}

	/// Retrieves the [ComponentType]'s unique runtime identifier.
	pub const fn id(&self) -> ComponentId {
		self.id
	}

	/// Retrieves the [ComponentType]'s unique compiletime identifier.
	pub const fn type_id(&self) -> TypeId {
		self.type_id
	}

	pub(crate) fn create_buffer(&self) -> AnyBuffer {
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

/// It provides a unified way to access a component's [ComponentId] and type
/// through its base type and all derived ref types.
///
/// This trait should only be implemented by #\[derive([`Component`])].
pub trait ComponentTypeInfo {
	/// The underlying [Component]'s type
	type ComponentType: ComponentTypeInfo;

	/// Retrieves the [Component] type's unique runtime identifier.
	fn component_id() -> ComponentId;
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

pub(crate) trait ComponentFrom<T> {
	/// # Safety
	/// Always safe if called by IterArchetype.
	/// IterArchetype's implementation guarantees Rust's aliasing rules are maintained.
	unsafe fn convert(value: T) -> Self;
}

impl<T: Component + Copy> ComponentFrom<*const T> for T {
	#[inline(always)]
	unsafe fn convert(value: *const T) -> Self {
		*value
	}
}

impl<T: Component + Copy> ComponentFrom<*mut T> for T {
	#[inline(always)]
	unsafe fn convert(value: *mut T) -> Self {
		*value
	}
}

impl<T: Component> ComponentFrom<*const T> for &T {
	#[inline(always)]
	unsafe fn convert(value: *const T) -> Self {
		&*value
	}
}

impl<T: Component> ComponentFrom<*mut T> for &T {
	#[inline(always)]
	unsafe fn convert(value: *mut T) -> Self {
		&*value
	}
}

impl<T: Component> ComponentFrom<*mut T> for &mut T {
	#[inline(always)]
	unsafe fn convert(value: *mut T) -> Self {
		&mut *value
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
