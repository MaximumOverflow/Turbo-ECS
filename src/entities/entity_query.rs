use crate::components::{ComponentSet};
use crate::data_structures::BitField;
use std::hash::BuildHasherDefault;
use nohash_hasher::NoHashHasher;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::marker::PhantomData;
use parking_lot::RwLock;
use std::any::TypeId;
use std::ops::Deref;
use std::sync::Arc;

type Hasher = BuildHasherDefault<NoHashHasher<u64>>;

lazy_static! {
	static ref QUERY_TO_DATA: RwLock<Vec<EntityQueryData>> = RwLock::new(Vec::default());
	static ref PTR_TO_QUERY: RwLock<HashMap<(usize, usize), EntityQuery>> = RwLock::new(HashMap::default());
	static ref TYPE_TO_QUERY: RwLock<HashMap<TypeId, EntityQuery, Hasher>> = RwLock::new(HashMap::default());
}

/// A handle to [BitField] based entity filter.
#[derive(Debug, Hash, Copy, Clone, Eq, PartialEq)]
pub struct EntityQuery {
	index: usize,
}

impl EntityQuery {
	pub fn build() -> QueryBuilder {
		QueryBuilder::default()
	}
}

/// A utility structure to build [EntityQueries](EntityQuery).
#[derive(Default)]
pub struct QueryBuilder<I: 'static + ComponentSet = (), E: 'static + ComponentSet = ()> {
	i_phantom: PhantomData<&'static I>,
	e_phantom: PhantomData<&'static E>,
}

impl<I: 'static + ComponentSet, E: 'static + ComponentSet> QueryBuilder<I, E> {
	/// Specify which types to include in the query.
	pub fn include<TI: 'static + ComponentSet>(self) -> QueryBuilder<TI, E> {
		QueryBuilder {
			i_phantom: PhantomData::default(),
			e_phantom: PhantomData::default(),
		}
	}

	/// Specify which types to exclude from the query.
	pub fn exclude<TE: 'static + ComponentSet>(self) -> QueryBuilder<I, TE> {
		QueryBuilder {
			i_phantom: PhantomData::default(),
			e_phantom: PhantomData::default(),
		}
	}

	/// Construct a query from the previously specified types.
	pub fn create(self) -> EntityQuery {
		<(I, E)>::get_query()
	}
}

/// This trait should only be implemented by #\[derive([`Component`])] for use by IterArchetype.
/// It provides a unified way to create an [EntityQuery] from a set of [Component] types through their base type and all derived ref types.
pub trait ComponentQuery {
	type Arguments;
	fn get_query() -> EntityQuery;
}

impl<I: 'static + ComponentSet, E: 'static + ComponentSet> ComponentQuery for (I, E) {
	type Arguments = I;
	#[inline(never)]
	fn get_query() -> EntityQuery {
		let key = TypeId::of::<Self>();
		let ttq = TYPE_TO_QUERY.read();
		match ttq.get(&key) {
			Some(query) => *query,
			None => {
				drop(ttq);
				create_query::<I, E>(key)
			},
		}
	}
}

#[derive(Clone)]
pub(crate) struct EntityQueryData {
	include: Arc<BitField>,
	exclude: Arc<BitField>,
}

impl EntityQueryData {
	pub fn include(&self) -> &BitField {
		&self.include
	}
	pub fn exclude(&self) -> &BitField {
		&self.exclude
	}
}

pub(crate) fn get_query_data(query: EntityQuery) -> EntityQueryData {
	let vec = QUERY_TO_DATA.read();
	vec[query.index].clone()
}

#[inline(never)]
fn create_query<I: 'static + ComponentSet, E: 'static + ComponentSet>(key: TypeId) -> EntityQuery {
	let mut ttq = TYPE_TO_QUERY.write();

	let (include, has_repeats) = I::get_bitfield();
	let (exclude, _) = E::get_bitfield();

	if has_repeats {
		panic!("An entity query cannot include a type multiple times")
	}

	let data = EntityQueryData { include, exclude };

	let ptr = (
		data.include.deref() as *const BitField as usize,
		data.exclude.deref() as *const BitField as usize,
	);

	let mut ptq = PTR_TO_QUERY.write();
	if let Some(query) = ptq.get(&ptr) {
		return *query;
	}

	let mut qtd = QUERY_TO_DATA.write();
	let query = EntityQuery { index: qtd.len() };

	qtd.push(data);
	ptq.insert(ptr, query);
	ttq.insert(key, query);
	query
}
