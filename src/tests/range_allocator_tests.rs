use crate::data_structures::RangeAllocator;
use rand::prelude::SliceRandom;
use rand::thread_rng;

#[test]
pub fn sequential_allocation() {
	let mut allocator = RangeAllocator::new();

	for i in 0..16 {
		let range = allocator.allocate(16);

		assert_eq!(
			range,
			i * 16..(i + 1) * 16,
			"Allocated range does not match expected range"
		);
		assert_eq!(
			(i + 1) * 16,
			allocator.capacity(),
			"Capacity does not match expected capacity"
		);
		assert_eq!(
			[0..(i + 1) * 16],
			allocator.used_ranges().collect::<Vec<_>>().as_slice(),
			"Used ranges do not match the expected ranges"
		);
	}
}

#[test]
pub fn sequential_deallocation() {
	let mut allocator = RangeAllocator::new();
	allocator.allocate(16 * 16);

	for i in 0..16 {
		let range = i * 16..(i + 1) * 16;
		allocator.free(range);

		let none = [].as_slice();
		let expected_free = 0..(i + 1) * 16;
		let expected_used = (i + 1) * 16..allocator.capacity();

		assert_eq!(
			if expected_free.is_empty() {
				none
			} else {
				std::slice::from_ref(&expected_free)
			},
			allocator.free_ranges().collect::<Vec<_>>().as_slice(),
			"Free ranges do not match the expected ranges"
		);
		assert_eq!(
			if expected_used.is_empty() {
				none
			} else {
				std::slice::from_ref(&expected_used)
			},
			allocator.used_ranges().collect::<Vec<_>>().as_slice(),
			"Used ranges do not match the expected ranges"
		);
	}
}

#[test]
pub fn fragmented_deallocation() {
	let count = 1024;

	let mut allocator = RangeAllocator::new();
	allocator.allocate(16 * count);

	let mut ranges = (0..count).map(|i| i * 16..(i + 1) * 16).collect::<Vec<_>>();
	ranges.shuffle(&mut thread_rng());

	for (i, range) in ranges.iter().enumerate() {
		allocator.free(range.clone());
		assert_eq!(
			(i + 1) * 16,
			allocator.available(),
			"Available space does not match expected space"
		);
	}

	assert_eq!(
		allocator.free_ranges().collect::<Vec<_>>().as_slice(),
		[0..allocator.capacity()],
		"Available space does not match expected space"
	);
}
