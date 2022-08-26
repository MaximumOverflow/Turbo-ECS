use std::sync::atomic::AtomicU32;
use std::hash::{Hash, Hasher};
use std::cmp::Ordering;
use std::iter::repeat;
use std::ops::Range;

const BITS: usize = 32;
const ALL_BITS_SET: u32 = u32::MAX;
const FIRST_BIT: u32 = 1 << (BITS - 1);

/// A dynamically sized bit-field.
#[derive(Default)]
pub struct BitField {
	values: Vec<u32>,
}

#[allow(unused)]
impl BitField {
	/// Create a new [BitField].
	pub fn new() -> Self {
		Self::default()
	}

	/// Create a new [BitField] with the specified capacity.
	///
	/// # Arguments
	/// * `capacity` - A usize representing the container's target capacity in bits
	pub fn with_capacity(capacity: usize) -> Self {
		let mut instance = Self { values: Vec::new() };
		instance.ensure_capacity(capacity);
		instance
	}

	/// Get the value of the bit at index `i`.
	///
	/// # Arguments
	/// * `i` - The index of the element to retrieve
	pub fn get(&self, i: usize) -> bool {
		self.get_inlined(i)
	}

	/// Set the value of the bit at index `i`.
	///
	/// # Arguments
	/// * `i` - The index of the element to modify
	pub fn set(&mut self, i: usize, value: bool) {
		self.set_inlined(i, value)
	}

	/// Get the value of the bit at index `i`.
	///
	/// # Arguments
	/// * `i` - The index of the element to retrieve
	#[inline(always)]
	pub fn get_inlined(&self, i: usize) -> bool {
		let (position, shift) = Self::pos_shift(i);
		match self.values.len().cmp(&position) {
			Ordering::Greater => {
				let bit_value = unsafe { self.values.get_unchecked(position) };
				let bit = FIRST_BIT >> shift;
				(bit_value & bit as u32) != 0
			},
			_ => false,
		}
	}

	/// Set the value of the bit at index `i`.
	///
	/// # Arguments
	/// * `i` - The index of the element to modify
	#[inline(always)]
	pub fn set_inlined(&mut self, i: usize, value: bool) {
		let (position, shift) = Self::pos_shift(i);
		let bit = FIRST_BIT >> shift;

		match value {
			true => {
				if self.values.len() <= position {
					self.extend_to_position(position);
				}
				let bit_value = unsafe { self.values.get_unchecked_mut(position) };
				*bit_value |= bit;
			},

			false => {
				if self.values.len() <= position {
					return;
				}
				let bit_value = unsafe { self.values.get_unchecked_mut(position) };
				*bit_value &= !bit;
			},
		}
	}

	/// Set the value of the bit at index `i`.
	///
	/// # Arguments
	/// * `i` - The index of the element to modify
	///
	/// # Safety
	/// Parameter `i` must be in range from 0 to \[capacity].
	#[inline(always)]
	pub unsafe fn set_inlined_unchecked(&mut self, i: usize, value: bool) {
		let (position, shift) = Self::pos_shift(i);
		let bit = FIRST_BIT >> shift;

		match value {
			true => {
				let bit_value = self.values.get_unchecked_mut(position);
				*bit_value |= bit;
			},

			false => {
				let bit_value = self.values.get_unchecked_mut(position);
				*bit_value &= !bit;
			},
		}
	}

	/// Atomically set the value of the bit at index `i`.
	///
	/// # Arguments
	/// * `i` - The index of the element to modify
	///
	/// # Safety
	/// Parameter `i` must be in range from 0 to \[capacity].
	#[inline(always)]
	pub unsafe fn set_inlined_unchecked_atomic(&mut self, i: usize, value: bool) {
		let (position, shift) = Self::pos_shift(i);
		let bit = FIRST_BIT >> shift;

		let values: &mut [AtomicU32] = std::mem::transmute(self.values.as_mut_slice());

		match value {
			true => {
				let bit_value = values.get_unchecked_mut(position);
				bit_value.fetch_or(bit, std::sync::atomic::Ordering::Relaxed);
			},

			false => {
				let bit_value = values.get_unchecked_mut(position);
				bit_value.fetch_and(!bit, std::sync::atomic::Ordering::Relaxed);
			},
		}
	}

	/// Set the value of the bits at the specified indices.
	///
	/// # Arguments
	/// * `indices` - The indices of the element to modify
	///
	/// # Safety
	/// All indices must be in range from 0 to \[capacity].
	pub unsafe fn set_batch_unchecked<const VALUE: bool>(&mut self, indices: &[usize]) {
		for i in indices {
			self.set_inlined_unchecked(*i, VALUE);
		}
	}

	/// Check if the [BitField] is a subset of another [BitField].
	///
	/// # Arguments
	/// * `other` - The bitfield to check against
	pub fn is_subset_of(&self, other: &BitField) -> bool {
		if self.values.is_empty() || other.values.is_empty() {
			return false;
		}
		self.values.iter().zip(other.values.iter()).any(|(mask, bits)| (*bits & *mask) == *mask)
	}

	/// Set all bits to 0.
	pub fn clear(&mut self) {
		self.values.fill(0);
	}

	/// Set the minimum capacity of the [BitField].
	/// # Arguments
	/// * `capacity` - A usize representing the container's minimum capacity in bits
	pub fn ensure_capacity(&mut self, capacity: usize) {
		if self.values.len() * BITS < capacity {
			let mut count = capacity / BITS;
			if count * BITS < capacity {
				count += 1;
			}
			count -= self.values.len();

			self.values.extend(repeat(0).take(count));
		}
	}

	/// Get the [BitField]'s capacity in bits.
	pub fn capacity(&self) -> usize {
		self.values.len() * BITS
	}

	/// Iterate over the ranges of set bits.
	pub fn iter_ranges(&self) -> BitFieldRangeIterator {
		BitFieldRangeIterator::new(&self.values)
	}

	/// Reserve an additional \[count] bits.
	///
	/// # Arguments
	/// * `count` - The minimum number of additional bits to reserve
	pub fn reserve(&mut self, count: usize) {
		let mut new = count / BITS;
		if new * BITS < count {
			new += 1;
		}
		self.values.extend(repeat(0).take(new));
	}

	#[inline(never)]
	fn extend_to_position(&mut self, position: usize) {
		let count = position - self.values.len() + 1;
		for _ in 0..count {
			self.values.push(0);
		}
	}

	#[inline(always)]
	fn pos_shift(a: usize) -> (usize, usize) {
		(a / BITS, a % BITS)
	}
}

impl From<&[u32]> for BitField {
	fn from(values: &[u32]) -> Self {
		Self {
			values: Vec::from(values),
		}
	}
}

impl Eq for BitField {}

impl PartialEq<Self> for BitField {
	fn eq(&self, other: &Self) -> bool {
		match self.values.len().cmp(&other.values.len()) {
			Ordering::Equal => self.values.eq(&other.values),
			Ordering::Less => {
				self.values.eq(&other.values[0..self.values.len()])
					&& other.values[self.values.len()..other.values.len()].iter().all(|i| *i == 0)
			},
			Ordering::Greater => {
				self.values[0..other.values.len()].eq(&other.values)
					&& self.values[other.values.len()..self.values.len()].iter().all(|i| *i == 0)
			},
		}
	}
}

impl Hash for BitField {
	fn hash<H: Hasher>(&self, state: &mut H) {
		let last = {
			let mut last = None;
			for i in (0..self.values.len()).rev() {
				if self.values[i] != 0 {
					last = Some(i);
					break;
				}
			}

			last
		};

		if let Some(last) = last {
			for i in &self.values[0..last] {
				i.hash(state);
			}
		}
	}
}

/// Iterates over the ranges of set bits of a [BitField].
pub struct BitFieldRangeIterator<'l> {
	index: usize,
	sub_index: u32,
	values: &'l [u32],
}

impl<'l> BitFieldRangeIterator<'l> {
	fn new(values: &'l [u32]) -> Self {
		Self {
			index: 0,
			sub_index: 0,
			values,
		}
	}
}

impl Iterator for BitFieldRangeIterator<'_> {
	type Item = Range<usize>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.index >= self.values.len() {
			return None;
		}

		while self.values[self.index] == 0 {
			self.index += 1;
			self.sub_index = 0;

			if self.index >= self.values.len() {
				return None;
			}
		}

		let value = self.values[self.index];
		let first_bit = match find_first_bit(value, self.sub_index) {
			None => return None,
			Some(bit) => bit,
		};

		let last_bit = find_last_bit(value, first_bit as u32);
		let start = self.index * BITS + first_bit;

		return match last_bit {
			Some(last_bit) => {
				let end = self.index * BITS + last_bit;
				self.sub_index = (last_bit + 1) as u32;
				Some(start..end)
			},
			None => {
				self.index += 1;
				let mut end = self.index * BITS;
				while self.index < self.values.len() {
					let value = self.values[self.index];
					if value == ALL_BITS_SET {
						end += BITS;
						self.index += 1;
					} else {
						let last_bit = find_last_bit(value, 0).unwrap();
						self.sub_index = (last_bit + 1) as u32;
						end += last_bit;
						return Some(start..end);
					}
				}

				Some(start..end)
			},
		};

		#[inline]
		fn find_first_bit(value: u32, start: u32) -> Option<usize> {
			let (mask, overflow) = u32::MAX.overflowing_shr(start);
			if overflow {
				return None;
			}
			let check = value & mask;
			match check {
				0 => None,
				_ => Some(check.leading_zeros() as usize),
			}
		}

		#[inline]
		fn find_last_bit(value: u32, start: u32) -> Option<usize> {
			let (mask, overflow) = u32::MAX.overflowing_shr(start);
			if overflow {
				return None;
			}
			let check = !value & mask;
			match check {
				0 => None,
				_ => Some(check.leading_zeros() as usize),
			}
		}
	}
}
