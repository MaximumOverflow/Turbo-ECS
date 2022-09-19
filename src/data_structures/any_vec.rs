use std::mem::{MaybeUninit, size_of};
use std::any::TypeId;
use std::ops::Range;

/// A polymorphic container for items of the same type.
/// The container does not keep track of which values stored within have been initialized,
/// nor will it automatically drop them upon destruction.
pub(crate) struct AnyBuffer {
	vec: Vec<u8>,
	type_id: TypeId,
	type_size: usize,
	drop: fn(&mut Self, Range<usize>),
	default: Option<fn(&mut Self, Range<usize>)>,
}

#[allow(dead_code)]
impl AnyBuffer {
	pub fn new<T: 'static>() -> Self {
		Self::with_capacity::<T>(0)
	}

	pub fn new_default<T: 'static + Default>() -> Self {
		Self::with_capacity_default::<T>(0)
	}

	#[allow(clippy::uninit_vec)]
	pub fn with_capacity<T: 'static>(capacity: usize) -> Self {
		unsafe {
			let len = size_of::<T>() * capacity;
			let mut vec = Vec::with_capacity(len);
			vec.set_len(vec.capacity());

			Self {
				vec,
				type_id: TypeId::of::<T>(),
				type_size: size_of::<T>(),

				drop: |this, range| {
					let ptr = (this.vec.as_mut_ptr() as *mut T).add(range.start);
					let slice = std::slice::from_raw_parts_mut(ptr, range.len());
					std::ptr::drop_in_place(slice);
				},

				default: None,
			}
		}
	}

	pub fn with_capacity_default<T: 'static + Default>(capacity: usize) -> Self {
		let mut this = Self::with_capacity::<T>(capacity);
		this.default = Some(|this, range| unsafe {
			let ptr = (this.vec.as_mut_ptr() as *mut T).add(range.start);
			let slice = std::slice::from_raw_parts_mut(ptr, range.len());
			for x in slice { std::ptr::write(x, T::default()); }
		});

		this
	}

	#[allow(clippy::uninit_vec)]
	pub fn ensure_capacity(&mut self, capacity: usize) {
		unsafe {
			let current = self.vec.len() / self.type_size;
			if current < capacity {
				let needed = capacity - current;
				self.vec.reserve(needed * self.type_size);
				self.vec.set_len(self.vec.capacity());
			}
		}
	}

	/// # Safety
	/// All values in `range` must be initialized.
	/// `range` must be within the bounds of the buffer.
	pub unsafe fn drop_values(&mut self, range: Range<usize>) {
		debug_assert!(range.start < self.capacity());
		debug_assert!(range.len() <= self.capacity() - range.start);

		(self.drop)(self, range);
	}

	/// # Safety
	/// All values in `range` must be dropped first.
	/// `range` must be within the bounds of the buffer.
	pub unsafe fn default_values(&mut self, range: Range<usize>) {
		debug_assert!(range.start < self.capacity());
		debug_assert!(range.len() <= self.capacity() - range.start);

		match self.default {
			None => panic!("Buffer does not have a default function for T"),
			Some(default) => default(self, range),
		}
	}

	pub fn as_slice<T: 'static>(&self) -> &[MaybeUninit<T>] {
		assert_eq!(self.type_id, TypeId::of::<T>(), "Buffer does not contain elements of type T");
		unsafe { self.as_slice_unchecked() }
	}

	pub unsafe fn as_slice_unchecked<T: 'static>(&self) -> &[T] {
		let ptr = self.vec.as_ptr() as *const T;
		std::slice::from_raw_parts(ptr, self.capacity())
	}

	pub fn as_mut_slice<T: 'static>(&mut self) -> &mut [MaybeUninit<T>] {
		assert_eq!(self.type_id, TypeId::of::<T>(), "Buffer does not contain elements of type T");
		unsafe { self.as_mut_slice_unchecked() }
	}

	pub unsafe fn as_mut_slice_unchecked<T: 'static>(&mut self) -> &mut [T] {
		let ptr = self.vec.as_mut_ptr() as *mut T;
		std::slice::from_raw_parts_mut(ptr, self.capacity())
	}

	pub fn capacity(&self) -> usize {
		self.vec.len() / self.type_size
	}
}
