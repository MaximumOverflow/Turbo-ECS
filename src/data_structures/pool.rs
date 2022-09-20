use std::ops::{Deref, DerefMut};
use std::mem::MaybeUninit;
use std::cell::RefCell;
use std::rc::Rc;

/// A simple object pool.
#[derive(Default)]
pub struct Pool<T: Default> {
	values: Rc<RefCell<Vec<T>>>,
}

/// A handle to a borrowed object in a [Pool]
pub struct PoolBorrow<T> {
	value: MaybeUninit<T>,
	values: Rc<RefCell<Vec<T>>>,
}

impl<T: Default> Pool<T> {
	/// Borrow one element from the pool.
	pub fn take_one(&mut self) -> PoolBorrow<T> {
		let value = self.values.deref().borrow_mut().pop().unwrap_or_else(|| T::default());
		PoolBorrow {
			value: MaybeUninit::new(value),
			values: self.values.clone(),
		}
	}
}

impl<T> Deref for PoolBorrow<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		unsafe { self.value.assume_init_ref() }
	}
}

impl<T> DerefMut for PoolBorrow<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { self.value.assume_init_mut() }
	}
}

impl<T> Drop for PoolBorrow<T> {
	fn drop(&mut self) {
		unsafe {
			let mut value = MaybeUninit::uninit();
			std::mem::swap(&mut value, &mut self.value);
			self.values.deref().borrow_mut().push(value.assume_init());
		}
	}
}
