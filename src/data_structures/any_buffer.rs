use std::mem::{MaybeUninit, align_of, size_of};
use std::alloc::Layout;
use std::any::TypeId;
use std::ops::Range;

/// A polymorphic container for items of the same type.
/// The container does not keep track of which values stored within have been initialized,
/// nor will it automatically drop them upon destruction.
pub(crate) struct AnyBuffer {
	buffer: Box<[u8]>,
	type_id: TypeId,
	type_size: usize,
	type_align: usize,
	drop: fn(&mut Self, Range<usize>),
	default: Option<fn(&mut Self, Range<usize>)>,
}

#[allow(dead_code)]
impl AnyBuffer {
	pub fn new<T: 'static>() -> Self {
		Self::with_capacity::<T>(1)
	}

	pub fn new_default<T: 'static + Default>() -> Self {
		Self::with_capacity_default::<T>(1)
	}

	#[allow(clippy::uninit_vec)]
	pub fn with_capacity<T: 'static>(capacity: usize) -> Self {
		unsafe {
			let type_size = size_of::<T>();
			let type_align = align_of::<T>();
			let buffer = make_buffer(type_size, type_align, capacity);

			Self {
				buffer,
				type_size,
				type_align,
				type_id: TypeId::of::<T>(),

				drop: |this, range| {
					let ptr = (this.buffer.as_mut_ptr() as *mut T).add(range.start);
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
			let ptr = (this.buffer.as_mut_ptr() as *mut T).add(range.start);
			let slice = std::slice::from_raw_parts_mut(ptr, range.len());
			for x in slice {
				std::ptr::write(x, T::default());
			}
		});

		this
	}

	#[allow(clippy::uninit_vec)]
	pub fn ensure_capacity(&mut self, capacity: usize) {
		unsafe {
			let current = self.capacity();
			if current < capacity {
				let mut buffer = make_buffer(self.type_size, self.type_align, capacity);
				std::ptr::copy_nonoverlapping(self.buffer.as_ptr(), buffer.as_mut_ptr(), self.buffer.len());
				self.buffer = buffer;
			}
		}
	}

	/// # Safety
	/// - All values in `range` must be initialized.
	/// - `range` must be within the bounds of the buffer.
	pub unsafe fn drop_values(&mut self, range: Range<usize>) {
		debug_assert!(range.start < self.capacity());
		debug_assert!(range.len() <= self.capacity() - range.start);

		(self.drop)(self, range);
	}

	/// # Safety
	/// - All values in `range` must be dropped first.
	/// - `range` must be within the bounds of the buffer.
	pub unsafe fn default_values(&mut self, range: Range<usize>) {
		debug_assert!(range.start < self.capacity());
		debug_assert!(range.len() <= self.capacity() - range.start);

		match self.default {
			None => panic!("Buffer does not have a default function for T"),
			Some(default) => default(self, range),
		}
	}

	/// # Safety
	/// - The two buffers must contain the same type.
	/// - `range` must be within the bounds of the buffer.
	/// - `det_offset` must be within the bounds of the destination buffer.
	/// - `range.len() + dst_offset` must be within the bounds of the destination buffer.
	pub unsafe fn copy_values(&mut self, dst: &mut Self, range: Range<usize>, dst_offset: usize) {
		debug_assert!(self.type_id == dst.type_id);

		debug_assert!(range.start < self.capacity());
		debug_assert!(range.len() <= self.capacity() - range.start);

		debug_assert!(dst_offset < dst.capacity());
		debug_assert!(range.len() <= dst.capacity() - dst_offset);

		let src = self.buffer.as_mut_ptr().add(range.start * self.type_size);
		let dst = dst.buffer.as_mut_ptr().add(dst_offset * self.type_size);
		std::ptr::copy_nonoverlapping(src, dst, range.len() * self.type_size);
	}

	pub fn as_slice<T: 'static>(&self) -> &[MaybeUninit<T>] {
		assert_eq!(
			self.type_id,
			TypeId::of::<T>(),
			"Buffer does not contain elements of type T"
		);
		unsafe { self.as_slice_unchecked() }
	}

	/// # Safety
	/// `T` must match the buffer's internal type.
	pub unsafe fn as_slice_unchecked<T: 'static>(&self) -> &[T] {
		let ptr = self.buffer.as_ptr() as *const T;
		std::slice::from_raw_parts(ptr, self.capacity())
	}

	pub fn as_mut_slice<T: 'static>(&mut self) -> &mut [MaybeUninit<T>] {
		assert_eq!(
			self.type_id,
			TypeId::of::<T>(),
			"Buffer does not contain elements of type T"
		);
		unsafe { self.as_mut_slice_unchecked() }
	}

	/// # Safety
	/// `T` must match the buffer's internal type.
	pub unsafe fn as_mut_slice_unchecked<T: 'static>(&mut self) -> &mut [T] {
		let ptr = self.buffer.as_mut_ptr() as *mut T;
		std::slice::from_raw_parts_mut(ptr, self.capacity())
	}

	pub fn capacity(&self) -> usize {
		self.buffer.len() / self.type_size
	}
}

unsafe fn make_buffer(t_size: usize, t_align: usize, count: usize) -> Box<[u8]> {
	let bytes = t_size.checked_mul(count).unwrap();
	let layout = Layout::from_size_align(bytes, t_align).unwrap();
	Box::from_raw(std::slice::from_raw_parts_mut(std::alloc::alloc(layout), layout.size()))
}
