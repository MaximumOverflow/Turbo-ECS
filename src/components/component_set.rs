use crate::components::ComponentTypeInfo;
use crate::data_structures::BitField;
use crate::components::ComponentId;
use std::hash::BuildHasherDefault;
use nohash_hasher::NoHashHasher;
use std::collections::HashMap;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::any::TypeId;
use std::sync::Arc;

type Hasher = BuildHasherDefault<NoHashHasher<u64>>;

lazy_static! {
	static ref EMPTY_BITFIELD: Arc<BitField> = Arc::new(BitField::new());
	static ref TYPE_TO_BITFIELD: Mutex<HashMap<TypeId, (Arc<BitField>, bool), Hasher>> = Mutex::new(HashMap::default());
	static ref VEC_TO_BITFIELD: Mutex<HashMap<Vec<ComponentId>, (Arc<BitField>, bool)>> = Mutex::new(HashMap::default());
}

/// This trait should only be implemented by #\[derive([`Component`])] for use by IterArchetype.
/// It provides a unified way to create a [BitField] from a set of [Component] types through their base type and all derived ref types.
pub trait ComponentSet {
	/// Extract a bitfield from a set of [ComponentIds](ComponentId)
	fn get_bitfield() -> (Arc<BitField>, bool);
}

impl ComponentSet for () {
	fn get_bitfield() -> (Arc<BitField>, bool) {
		(EMPTY_BITFIELD.clone(), false)
	}
}

fn make_bitfield(components: &[ComponentId]) -> (Arc<BitField>, bool) {
	let mut bitfield = BitField::new();
	let mut has_repeats = false;

	for component in components {
		has_repeats |= bitfield.get(component.value());
		bitfield.set(component.value(), true);
	}

	(Arc::new(bitfield), has_repeats)
}

macro_rules! impl_component_bitfield {
    ($($t: ident $i: tt),*) => {
        #[allow(unused_parens)]
        impl <$($t: 'static + ComponentTypeInfo),*> ComponentSet for ($($t),*,) {
            fn get_bitfield() -> (Arc<BitField>, bool) {
                let key = TypeId::of::<Self>();
                let mut ttb = TYPE_TO_BITFIELD.lock();
                if let Some((bitfield, repeats)) = ttb.get(&key) {
                    return (bitfield.clone(), *repeats)
                }

                let mut components = vec![$(<$t>::component_id()),*];
                components.sort_by_key(|a| a.value());

                let mut vtb = VEC_TO_BITFIELD.lock();
                if let Some((bitfield, repeats)) = vtb.get(&components) {
                    ttb.insert(key, (bitfield.clone(), *repeats));
                    return (bitfield.clone(), *repeats);
                }

                let (bitfield, repeats) = make_bitfield(components.as_slice());
                vtb.insert(components, (bitfield.clone(), repeats));
                ttb.insert(key, (bitfield.clone(), repeats));
                (bitfield, repeats)
            }
        }
    };
}

impl_component_bitfield!(T0 0);
impl_component_bitfield!(T0 0, T1 1);
impl_component_bitfield!(T0 0, T1 1, T2 2);
impl_component_bitfield!(T0 0, T1 1, T2 2, T3 3);
impl_component_bitfield!(T0 0, T1 1, T2 2, T3 3, T4 4);
impl_component_bitfield!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5);
impl_component_bitfield!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6);
impl_component_bitfield!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7);
impl_component_bitfield!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8);
impl_component_bitfield!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9);
impl_component_bitfield!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9, T10 10);
impl_component_bitfield!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5, T6 6, T7 7, T8 8, T9 9, T10 10, T11 11);
