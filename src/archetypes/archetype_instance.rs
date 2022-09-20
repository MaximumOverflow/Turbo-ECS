use crate::components::{Component, ComponentFrom, ComponentType, ComponentTypeInfo};
use crate::data_structures::{AnyBuffer, BitField, RangeAllocator, UsedRangeIterator};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::hash::BuildHasherDefault;
use nohash_hasher::NoHashHasher;
use std::collections::HashMap;
use std::any::TypeId;
use std::ops::Range;
use paste::paste;

type Hasher = BuildHasherDefault<NoHashHasher<u64>>;

#[derive(Default, Eq, PartialEq, Copy, Clone)]
pub struct Archetype {
	pub(crate) index: usize,
}

pub struct ArchetypeInstance {
	bitfield: BitField,
	allocator: RangeAllocator,
	component_bitfield: BitField,
	components: Vec<ComponentType>,
	buffers: HashMap<TypeId, AnyBuffer, Hasher>,
}

impl ArchetypeInstance {
	pub(crate) fn new(components: &[ComponentType]) -> Self {
		Self::with_capacity(components, 0)
	}

	pub(crate) fn with_capacity(components: &[ComponentType], capacity: usize) -> Self {
		let mut component_bitfield = BitField::new();
		let bitfield = BitField::with_capacity(capacity);
		let allocator = RangeAllocator::with_capacity(capacity);

		let buffers = HashMap::from_iter(components.iter().filter_map(|t| {
			let index = t.id().value();
			if component_bitfield.get(index) {
				None
			} else {
				let mut vec = t.create_buffer();
				vec.ensure_capacity(capacity);

				component_bitfield.set(index, true);
				Some((t.type_id(), vec))
			}
		}));

		Self {
			buffers,
			bitfield,
			allocator,
			component_bitfield,
			components: components.into(),
		}
	}

	/// Allocate \[count] slots, setting all components to their default value.
	/// The returned slot chunks might be fragmented.
	///
	/// # Arguments
	/// * `count` - The amount of slots to allocate
	/// * `ranges` - The allocated chunks will be pushed here
	pub fn take_slots(&mut self, count: usize, ranges: &mut Vec<Range<usize>>) {
		self.take_slots_no_init(count, ranges);
		for buffer in self.buffers.values_mut() {
			for range in ranges.iter() {
				unsafe {
					buffer.default_values(range.clone());
				}
			}
		}
	}

	/// Allocate \[count] slots.
	/// The returned slot chunks might be fragmented.
	///
	/// # Arguments
	/// * `count` - The amount of slots to allocate
	/// * `ranges` - The allocated chunks will be pushed here
	pub fn take_slots_no_init(&mut self, count: usize, ranges: &mut Vec<Range<usize>>) {
		ranges.clear();
		match self.allocator.try_allocate_fragmented(count, ranges) {
			Ok(_) => {},
			Err(needed) => {
				for buffer in self.buffers.values_mut() {
					buffer.ensure_capacity(self.allocator.capacity() + needed);
				}
				self.allocator.allocate_fragmented(count, ranges);
				self.bitfield.ensure_capacity(self.allocator.capacity());
			},
		};
	}

	/// # Safety
	/// All slots must be within range from 0 to `capacity`.
	/// Repeated values are allowed.
	pub unsafe fn return_slots(&mut self, slots: &[usize]) {
		self.bitfield.clear();
		self.bitfield.set_batch_unchecked::<true>(slots);
		for range in self.bitfield.iter_ranges() {
			for buffer in self.buffers.values_mut() {
				buffer.drop_values(range.clone());
			}
			self.allocator.free(range);
		}
	}

	/// # Safety
	/// All slots must be within range from 0 to `capacity`.
	/// Repeated values are allowed.
	pub unsafe fn return_slots_no_drop(&mut self, slots: &[usize]) {
		self.bitfield.clear();
		self.bitfield.set_batch_unchecked::<true>(slots);
		for range in self.bitfield.iter_ranges() {
			self.allocator.free(range);
		}
	}

	pub fn matches_query(&self, set: &BitField) -> bool {
		set.is_subset_of(&self.component_bitfield)
	}

	pub fn ensure_capacity(&mut self, capacity: usize) {
		if self.allocator.capacity() < capacity {
			self.bitfield.ensure_capacity(capacity);
			self.allocator.ensure_capacity(capacity);
			for buffer in self.buffers.values_mut() {
				buffer.ensure_capacity(capacity);
			}
		}
	}

	pub fn get_component<T: 'static + Component>(&self, slot: usize) -> Option<&T> {
		unsafe {
			let buffer = self.buffers.get(&TypeId::of::<T>())?;
			let vec = buffer.as_slice_unchecked::<T>();
			Some(vec.get_unchecked(slot))
		}
	}

	pub fn get_component_mut<T: 'static + Component>(&mut self, slot: usize) -> Option<&mut T> {
		unsafe {
			let buffer = self.buffers.get_mut(&TypeId::of::<T>())?;
			let vec = buffer.as_mut_slice_unchecked::<T>();
			Some(vec.get_unchecked_mut(slot))
		}
	}

	pub fn iter_used_ranges(&self) -> UsedRangeIterator {
		self.allocator.used_ranges()
	}

	pub fn components(&self) -> &[ComponentType] {
		&self.components
	}

	pub fn component_bitfield(&self) -> &BitField {
		&self.component_bitfield
	}

	pub unsafe fn copy_components(&self, dst: &mut ArchetypeInstance, src_idx: usize, dst_idx: usize) {
		for (key, src) in self.buffers.iter() {
			if let Some(dst) = dst.buffers.get_mut(key) {
				dst.drop_values(dst_idx..dst_idx + 1);
				src.copy_values(dst, src_idx..src_idx + 1, dst_idx);
			}
		}
	}
}

impl Drop for ArchetypeInstance {
	fn drop(&mut self) {
		unsafe {
			for buffer in self.buffers.values_mut() {
				for range in self.allocator.used_ranges() {
					buffer.drop_values(range)
				}
			}
		}
	}
}

pub trait IterateArchetype<T> {
	fn for_each_mut(&mut self, func: &mut impl FnMut(T));
}

pub trait IterateArchetypeParallel<T> {
	fn for_each_mut(&mut self, func: &(impl Fn(T) + Send + Sync));
}

impl IterateArchetype<()> for ArchetypeInstance {
	fn for_each_mut(&mut self, _: &mut impl FnMut(())) {}
}

macro_rules! impl_archetype_iter {
    ($($t: ident),*) => {
        paste! {
            #[allow(unused_parens)]
            impl <$($t: 'static + ComponentTypeInfo + ComponentFrom<*mut $t::ComponentType>),*> IterateArchetype<($($t),*)> for ArchetypeInstance
			{
                fn for_each_mut(&mut self, func: &mut impl FnMut(($($t),*))) {
                    unsafe {
                        $(
                            let [<$t:lower>] = self.buffers.get_mut(&TypeId::of::<$t::ComponentType>()).unwrap();
                            let [<$t:lower>] = [<$t:lower>].as_mut_slice_unchecked::<$t::ComponentType>().as_mut_ptr();
                        )*
                        for range in self.allocator.used_ranges() {
                            for i in range {
                                $(let [<$t:lower>] = [<$t:lower>].add(i);)*
                                func(($($t::convert([<$t:lower>])),*));
                            }
                        }
                    }
                }
            }

			#[allow(unused_parens)]
			impl<$($t: 'static + ComponentTypeInfo + ComponentFrom<*mut $t::ComponentType> + Send + Sync),*> IterateArchetypeParallel<($($t),*)> for ArchetypeInstance
			{
				fn for_each_mut(&mut self, func: &(impl Fn(($($t),*)) + Sync + Send)) {
					unsafe {
						$(
                            let [<$t:lower>] = self.buffers.get_mut(&TypeId::of::<$t::ComponentType>()).unwrap();
                            let [<$t:lower>] = [<$t:lower>].as_mut_slice_unchecked::<$t::ComponentType>().as_mut_ptr() as usize;
                        )*

						let ranges: Vec<_> = self.allocator.used_ranges().collect();
						ranges.into_par_iter().flatten().for_each(|i| {
							$(let [<$t:lower>] = ([<$t:lower>] as *mut $t::ComponentType).add(i);)*
							func(($($t::convert([<$t:lower>])),*));
						});
					}
				}
			}

        }
    };
}

impl_archetype_iter!(T0);
impl_archetype_iter!(T0, T1);
impl_archetype_iter!(T0, T1, T2);
impl_archetype_iter!(T0, T1, T2, T3);
impl_archetype_iter!(T0, T1, T2, T3, T4);
impl_archetype_iter!(T0, T1, T2, T3, T4, T5);
impl_archetype_iter!(T0, T1, T2, T3, T4, T5, T6);
impl_archetype_iter!(T0, T1, T2, T3, T4, T5, T6, T7);
impl_archetype_iter!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_archetype_iter!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_archetype_iter!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_archetype_iter!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
