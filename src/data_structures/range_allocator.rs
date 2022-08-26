use std::collections::btree_map::Values;
use std::collections::BTreeMap;
use std::iter::Cloned;

type Range = std::ops::Range<usize>;

/// A simple memory management utility.
#[derive(Default)]
pub struct RangeAllocator {
	used: usize,
	capacity: usize,
	ranges: BTreeMap<usize, Range>,
}

impl RangeAllocator {
	/// Create a new [RangeAllocator]
	pub fn new() -> Self {
		Self::default()
	}

	/// Create a new [RangeAllocator] with the specified capacity.
	///
	/// # Arguments
	/// * `capacity` - A usize representing the container's target capacity
	pub fn with_capacity(capacity: usize) -> Self {
		if capacity == 0 {
			Self::default()
		} else {
			Self {
				used: 0,
				capacity,
				ranges: BTreeMap::from_iter([(0, 0..capacity)]),
			}
		}
	}

	/// Allocate a continuous chunk of size \[size].
	///
	/// # Arguments
	/// * `size` - The size of the chunk to allocate
	pub fn allocate(&mut self, size: usize) -> Range {
		match self.try_allocate(size) {
			Ok(range) => range,
			Err(_) => self.allocate_new(size),
		}
	}

	/// Conditionally allocate a continuous chunk of size \[size].
	/// The function returns None if there are no available chunks to allocate into.
	///
	/// # Arguments
	/// * `size` - The size of the chunk to allocate
	pub fn try_allocate(&mut self, size: usize) -> Result<Range, usize> {
		let find =
			self.ranges.iter().find_map(|(k, r)| if r.len() >= size { Some(k) } else { None });

		match find {
			Some(start) => {
				let start = *start;
				let used_range = start..start + size;
				let mut range = self.ranges.get(&start).unwrap().clone();
				range.start += size;

				self.ranges.remove(&start);
				if !range.is_empty() {
					self.ranges.insert(range.start, range);
				}

				self.used += size;
				Ok(used_range)
			},
			None => Err(size - self.available()),
		}
	}

	/// Allocate multiple chunks adding up to a size of \[size].
	///
	/// # Arguments
	/// * `size` - The total amount of space to allocate
	/// * `ranges` - The allocated ranges will be outputted here
	pub fn allocate_fragmented(&mut self, size: usize, ranges: &mut Vec<Range>) {
		let mut remaining = size;

		for range in self.ranges.values() {
			if remaining == 0 {
				break;
			}

			if range.len() < remaining {
				ranges.push(range.clone());
				remaining -= range.len();
				self.used += range.len();
			} else {
				let mut new_range = range.clone();
				new_range.start += remaining;

				let start = range.start;
				self.ranges.remove(&start);

				if !new_range.is_empty() {
					self.ranges.insert(new_range.start, new_range);
				}

				ranges.push(start..start + remaining);
				self.used += remaining;
				remaining = 0;
				break;
			}
		}

		if remaining != 0 {
			if !ranges.is_empty() {
				let idx = ranges.len() - 1;
				let range = self.allocate_new(remaining);
				if ranges[idx].end == range.start {
					ranges[idx].end = range.end;
				} else {
					ranges.push(range);
				}
			} else {
				ranges.push(self.allocate_new(remaining));
			}
		}

		for x in ranges.iter() {
			self.ranges.remove(&x.start);
		}
	}

	/// Conditionally allocate multiple chunks adding up to a size of \[size].
	/// The function will return the amount of additional space required for a successful allocation
	/// if there's not enough space available.
	///
	/// # Arguments
	/// * `size` - The total amount of space to allocate
	/// * `ranges` - The allocated ranges will be outputted here
	pub fn try_allocate_fragmented(
		&mut self, size: usize, ranges: &mut Vec<Range>,
	) -> Result<(), usize> {
		if self.available() < size {
			Err(size - self.available())
		} else {
			self.allocate_fragmented(size, ranges);
			Ok(())
		}
	}

	/// Return a range to the allocator.
	///
	/// # Arguments
	/// * `range` - The range to be returned to the allocator. Ranges should never be returned twice.
	//noinspection DuplicatedCode
	pub fn free(&mut self, range: Range) {
		if range.is_empty() {
			return;
		}
		let find_start = self.ranges.get(&range.end);
		match find_start {
			None => {},
			Some(extend) => {
				let key = extend.start;
				let mut extend = extend.clone();

				extend.start -= range.len();

				self.used -= range.len();
				self.ranges.remove(&key);

				let find_end =
					self.ranges
						.iter()
						.find_map(|(k, r)| if r.end == extend.start { Some(k) } else { None });
				match find_end {
					None => {
						self.ranges.insert(extend.start, extend.clone());
						return;
					},

					Some(key) => {
						let key = *key;
						let range = self.ranges.get_mut(&key).unwrap();
						range.end = extend.end;
						return;
					},
				}
			},
		}

		let find_end =
			self.ranges.iter().find_map(|(k, r)| if r.end == range.start { Some(k) } else { None });
		match find_end {
			None => {},
			Some(key) => {
				let key = *key;
				let extend = self.ranges.get_mut(&key).unwrap();

				self.used -= range.len();
				extend.end += range.len();
				return;
			},
		}

		self.used -= range.len();
		self.ranges.insert(range.start, range);
	}

	/// Get the amount of available space left to the allocator.
	pub fn available(&self) -> usize {
		self.capacity - self.used
	}

	/// Get the total capacity of the allocator.
	pub fn capacity(&self) -> usize {
		self.capacity
	}

	/// Set the minimum capacity of the allocator.
	/// # Arguments
	/// * `capacity` - A usize representing the allocator's minimum capacity
	pub fn ensure_capacity(&mut self, capacity: usize) {
		if capacity > self.capacity {
			let count = capacity - self.capacity;
			self.reserve(count);
		}
	}

	/// Reserve an additional chunk of size \[size].
	/// # Arguments
	/// * `size` - The size of the chunk to reserve
	pub fn reserve(&mut self, size: usize) {
		let start = self.capacity;
		self.capacity += size;
		self.ranges.insert(start, start..self.capacity);
	}

	/// Iterate over the unallocated chunks
	pub fn free_ranges(&self) -> Cloned<Values<usize, Range>> {
		self.ranges.values().cloned()
	}

	/// Iterate over the allocated chunks
	pub fn used_ranges(&self) -> UsedRangeIterator {
		UsedRangeIterator::new(self)
	}

	fn allocate_new(&mut self, size: usize) -> Range {
		let start = self.capacity;
		self.capacity += size;
		self.used += size;
		start..self.capacity
	}
}

/// Iterates over the allocated chunks of a [RangeAllocator]
pub struct UsedRangeIterator<'l> {
	lst: usize,
	cap: usize,
	itr: Values<'l, usize, Range>,
}

impl<'l> UsedRangeIterator<'l> {
	fn new(allocator: &'l RangeAllocator) -> Self {
		Self {
			lst: 0,
			cap: allocator.capacity,
			itr: allocator.ranges.values(),
		}
	}
}

impl Iterator for UsedRangeIterator<'_> {
	type Item = Range;
	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.itr.next() {
				None if self.lst != self.cap => {
					let range = self.lst..self.cap;
					self.lst = self.cap;
					return Some(range);
				},

				None => return None,

				Some(free) => {
					let range = self.lst..free.start;
					self.lst = free.end;

					if !range.is_empty() {
						return Some(range);
					} else {
						continue;
					}
				},
			}
		}
	}
}
