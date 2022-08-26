use crate::components::{ComponentSet};
use crate::data_structures::BitField;
use std::hash::BuildHasherDefault;
use nohash_hasher::NoHashHasher;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::marker::PhantomData;
use parking_lot::Mutex;
use std::any::TypeId;
use std::ops::Deref;
use std::sync::Arc;

type Hasher = BuildHasherDefault<NoHashHasher<u64>>;

lazy_static! {
	static ref QUERY_TO_DATA: Mutex<Vec<EntityQueryData>> = Mutex::new(Vec::default());
	static ref PTR_TO_QUERY: Mutex<HashMap<(usize, usize), EntityQuery>> =
		Mutex::new(HashMap::default());
	static ref TYPE_TO_QUERY: Mutex<HashMap<TypeId, EntityQuery, Hasher>> =
		Mutex::new(HashMap::default());
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
	pub fn get_query(self) -> EntityQuery {
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
	fn get_query() -> EntityQuery {
		let key = TypeId::of::<Self>();
		let mut ttq = TYPE_TO_QUERY.lock();
		match ttq.get(&key) {
			Some(query) => *query,
			None => create_query::<I, E>(key, &mut ttq),
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
	let vec = QUERY_TO_DATA.lock();
	vec[query.index].clone()
}

#[inline(never)]
fn create_query<I: 'static + ComponentSet, E: 'static + ComponentSet>(
	key: TypeId, ttq: &mut HashMap<TypeId, EntityQuery, Hasher>,
) -> EntityQuery {
	let data = EntityQueryData {
		include: I::get_bitfield(),
		exclude: E::get_bitfield(),
	};

	let ptr = (
		data.include.deref() as *const BitField as usize,
		data.exclude.deref() as *const BitField as usize,
	);

	let mut ptq = PTR_TO_QUERY.lock();
	if let Some(query) = ptq.get(&ptr) {
		return *query;
	}

	let mut qtd = QUERY_TO_DATA.lock();
	let query = EntityQuery { index: qtd.len() };

	qtd.push(data);
	ptq.insert(ptr, query);
	ttq.insert(key, query);
	query
}
