use std::collections::btree_map::Values;
use std::collections::BTreeMap;

type Range = std::ops::Range<usize>;

#[derive(Default)]
pub struct RangeAllocator {
	used: usize,
	capacity: usize,
	ranges: BTreeMap<usize, Range>,
}

impl RangeAllocator {
	pub fn new() -> Self {
		Self::default()
	}

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

	pub fn allocate(&mut self, size: usize) -> Range {
		match self.try_allocate(size) {
			Ok(range) => range,
			Err(_) => self.allocate_new(size),
		}
	}

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

	pub fn available(&self) -> usize {
		self.capacity - self.used
	}

	pub fn capacity(&self) -> usize {
		self.capacity
	}

	pub fn ensure_capacity(&mut self, capacity: usize) {
		if capacity > self.capacity {
			let count = capacity - self.capacity;
			self.reserve(count);
		}
	}

	pub fn reserve(&mut self, size: usize) {
		let start = self.capacity;
		self.capacity += size;
		self.ranges.insert(start, start..self.capacity);
	}

	pub fn free_ranges(&self) -> Values<usize, Range> {
		self.ranges.values()
	}

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
			itr: allocator.free_ranges(),
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
