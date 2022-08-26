use std::iter::repeat;
use std::ops::Range;
use std::any::Any;

pub struct AnyVec {
	vec: Box<dyn Any>,

	get: fn(&Self, usize) -> &dyn Any,
	set: fn(&mut Self, usize, &dyn Any),

	clr: fn(&mut Self, usize),
	blk_clr: fn(&mut Self, Range<usize>),

	set_capacity: fn(&mut Self, usize),
}

impl AnyVec {
	pub fn new<T: 'static + Copy + Default>() -> Self {
		Self::with_capacity::<T>(0)
	}

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

	pub fn get_vec<T: 'static>(&self) -> Option<&Vec<T>> {
		self.vec.downcast_ref()
	}

	pub fn get_vec_mut<T: 'static>(&mut self) -> Option<&mut Vec<T>> {
		self.vec.downcast_mut()
	}

	/// # Safety
	/// This function expects T to match the internal Vec's element type
	pub unsafe fn get_vec_unchecked<T: 'static>(&self) -> &Vec<T> {
		&*(self.vec.as_ref() as *const dyn Any as *const Vec<T>)
	}

	/// # Safety
	/// `T` must the internal Vec's element type
	pub unsafe fn get_vec_mut_unchecked<T: 'static>(&mut self) -> &mut Vec<T> {
		&mut *(self.vec.as_mut() as *mut dyn Any as *mut Vec<T>)
	}

	pub fn get_value<T: 'static>(&self, i: usize) -> Option<&T> {
		let vec = self.get_vec()?;
		Some(&vec[i])
	}

	pub fn get_value_dyn(&self, i: usize) -> &dyn Any {
		(self.get)(self, i)
	}

	pub fn set_value_dyn(&mut self, i: usize, v: &dyn Any) {
		(self.set)(self, i, v)
	}

	pub fn clear_value(&mut self, i: usize) {
		(self.clr)(self, i)
	}

	pub fn clear_values(&mut self, range: Range<usize>) {
		(self.blk_clr)(self, range)
	}

	pub fn ensure_capacity(&mut self, capacity: usize) {
		(self.set_capacity)(self, capacity);
	}
}
