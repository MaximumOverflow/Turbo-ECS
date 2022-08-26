use std::iter::repeat;
use std::ops::Range;
use std::any::Any;

/// A polymorphic container items of the same type.
pub struct AnyVec {
	vec: Box<dyn Any>,

	get: fn(&Self, usize) -> &dyn Any,
	set: fn(&mut Self, usize, &dyn Any),

	clr: fn(&mut Self, usize),
	blk_clr: fn(&mut Self, Range<usize>),

	set_capacity: fn(&mut Self, usize),
}

impl AnyVec {
	/// Create a new [AnyVec] for items of type `T`.
	pub fn new<T: 'static + Copy + Default>() -> Self {
		Self::with_capacity::<T>(0)
	}

	/// Create a new [AnyVec] for items of type `T` with the specified capacity.
	///
	/// # Arguments
	/// * `capacity` - A usize representing the container's target capacity
	pub fn with_capacity<T: 'static + Copy + Default>(capacity: usize) -> Self {
		Self {
			vec: Box::new(vec![T::default(); capacity]),

			get: |this, i| unsafe { &this.get_vec_unchecked::<T>()[i] },
			set: |this, i, v| unsafe {
				this.get_vec_mut_unchecked::<T>()[i] = *v.downcast_ref::<T>().unwrap();
			},

			clr: |this: &mut AnyVec, i| unsafe {
				this.get_vec_mut_unchecked::<T>()[i] = T::default();
			},
			blk_clr: |this: &mut AnyVec, range| unsafe {
				let slice = &mut this.get_vec_mut_unchecked::<T>().as_mut_slice()[range];
				slice.fill(T::default());
			},

			set_capacity: |this, c| unsafe {
				let vec = this.get_vec_mut_unchecked::<T>();
				let count = c - vec.len();
				vec.extend(repeat(T::default()).take(count))
			},
		}
	}

	/// Get the a reference to the underlying [Vec].
	pub fn get_vec<T: 'static>(&self) -> Option<&Vec<T>> {
		self.vec.downcast_ref()
	}

	/// Get the a mutable reference to the underlying [Vec].
	pub fn get_vec_mut<T: 'static>(&mut self) -> Option<&mut Vec<T>> {
		self.vec.downcast_mut()
	}

	/// # Safety
	/// This function expects `T` to match the internal Vec's element type.
	pub unsafe fn get_vec_unchecked<T: 'static>(&self) -> &Vec<T> {
		&*(self.vec.as_ref() as *const dyn Any as *const Vec<T>)
	}

	/// # Safety
	/// `T` must the internal [Vec]'s element type.
	pub unsafe fn get_vec_mut_unchecked<T: 'static>(&mut self) -> &mut Vec<T> {
		&mut *(self.vec.as_mut() as *mut dyn Any as *mut Vec<T>)
	}

	/// Get a reference to the element at index `i`.
	///
	/// # Arguments
	/// * `i` - The index of the element to retrieve
	pub fn get_value<T: 'static>(&self, i: usize) -> Option<&T> {
		let vec = self.get_vec()?;
		Some(&vec[i])
	}

	/// Get a polymorphic reference to the element at index `i`.
	///
	/// # Arguments
	/// * `i` - The index of the element to retrieve
	pub fn get_value_dyn(&self, i: usize) -> &dyn Any {
		(self.get)(self, i)
	}

	/// Set the value of element at index `i`.
	///
	/// # Arguments
	/// * `i` - The index of the element to modify
	/// * `v` - The value to set the element to
	pub fn set_value_dyn(&mut self, i: usize, v: &dyn Any) {
		(self.set)(self, i, v)
	}

	/// Set the element at index `i` to its default value.
	///
	/// # Arguments
	/// * `i` - The index of the element to clear
	pub fn clear_value(&mut self, i: usize) {
		(self.clr)(self, i)
	}

	/// Set the elements in `range` to their default value.
	///
	/// # Arguments
	/// * `range` - The range of the elements to clear
	pub fn clear_values(&mut self, range: Range<usize>) {
		(self.blk_clr)(self, range)
	}

	/// Set the minimum capacity of the underlying [Vec].
	/// # Arguments
	/// * `capacity` - A usize representing the container's minimum capacity
	pub fn ensure_capacity(&mut self, capacity: usize) {
		(self.set_capacity)(self, capacity);
	}
}
