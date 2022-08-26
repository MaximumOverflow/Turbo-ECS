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
	static ref TYPE_TO_BITFIELD: Mutex<HashMap<TypeId, Arc<BitField>, Hasher>> =
		Mutex::new(HashMap::default());
	static ref VEC_TO_BITFIELD: Mutex<HashMap<Vec<ComponentId>, Arc<BitField>>> =
		Mutex::new(HashMap::default());
}

/// This trait should only be implemented by #\[derive([`Component`])] for use by IterArchetype.
/// It provides a unified way to create a [BitField] from a set of [Component] types through their base type and all derived ref types.
pub trait ComponentSet {
	/// Extract a bitfield from a set of [ComponentIds](ComponentId)
	fn get_bitfield() -> Arc<BitField>;
}

impl ComponentSet for () {
	fn get_bitfield() -> Arc<BitField> {
		EMPTY_BITFIELD.clone()
	}
}

macro_rules! impl_component_bitfield {
    ($($t: ident $i: tt),*) => {
        #[allow(unused_parens)]
        impl <$($t: 'static + ComponentTypeInfo),*> ComponentSet for ($($t),*,) {
            fn get_bitfield() -> Arc<BitField> {
                let key = TypeId::of::<Self>();
                let mut ttb = TYPE_TO_BITFIELD.lock();
                if let Some(bitfield) = ttb.get(&key) {
                    return bitfield.clone()
                }

                let mut components = vec![$(<$t>::component_id()),*];
                components.sort_by_key(|a| a.value());

                let mut vtb = VEC_TO_BITFIELD.lock();
                if let Some(bitfield) = vtb.get(&components) {
                    ttb.insert(key, bitfield.clone());
                    return bitfield.clone();
                }

                let bitfield = Arc::new(BitField::from(components.as_slice()));
                vtb.insert(components, bitfield.clone());
                ttb.insert(key, bitfield.clone());
                bitfield
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
