use crate::archetypes::{Archetype, ArchetypeInstance};
use crate::components::{ComponentId, ComponentType};
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;
use crate::entities::EntityQuery;
use nohash_hasher::NoHashHasher;

type Hasher = BuildHasherDefault<NoHashHasher<usize>>;

pub struct ArchetypeStore {
	vec: Vec<ArchetypeInstance>,
	map: HashMap<Vec<ComponentId>, Archetype>,
	queries: HashMap<EntityQuery, Vec<usize>, Hasher>,
}

impl ArchetypeStore {
	pub(crate) fn new() -> Self {
		Self {
			queries: HashMap::default(),
			vec: vec![ArchetypeInstance::new(&[])],
			map: HashMap::from([(vec![], Archetype::default())]),
		}
	}

	/// Creates an [`archetype`](Archetype) containing the specified [`components`](Component).
	pub fn create_archetype(&mut self, components: &[ComponentType]) -> Archetype {
		self.create_archetype_with_capacity(components, 0)
	}

	/// Creates an [`archetype`](Archetype) containing the specified [`components`](Component) with the specified capacity.
	#[inline(never)]
	pub fn create_archetype_with_capacity(
		&mut self, components: &[ComponentType], min_capacity: usize,
	) -> Archetype {
		let set = HashSet::<ComponentId>::from_iter(components.iter().map(|i| i.id()));
		let set: Vec<_> = set.iter().copied().collect();

		if let Some(archetype) = self.map.get(&set) {
			self.vec[archetype.index as usize].ensure_capacity(min_capacity);
			return *archetype;
		}

		let instance = ArchetypeInstance::with_capacity(components, min_capacity);
		let archetype = Archetype {
			index: self.vec.len(),
		};

		// Match archetype against all queries
		for (query, results) in self.queries.iter_mut() {
			let data = crate::entities::get_query_data(*query);
			if !instance.matches_query(data.include()) {
				continue;
			}
			if instance.matches_query(data.exclude()) {
				continue;
			}
			results.push(self.vec.len());
		}

		self.map.insert(set, archetype);
		self.vec.push(instance);
		archetype
	}

	pub(crate) fn get(&self, index: usize) -> &ArchetypeInstance {
		&self.vec[index]
	}

	pub(crate) fn get_mut(&mut self, index: usize) -> &mut ArchetypeInstance {
		&mut self.vec[index]
	}

	pub(crate) fn query(
		&mut self, query: EntityQuery,
	) -> impl Iterator<Item = &mut ArchetypeInstance> {
		if self.queries.get(&query).is_none() {
			self.init_query(query);
		}

		unsafe {
			let instances = self.vec.as_mut_ptr();
			self.queries.get(&query).unwrap().iter().map(move |i| &mut *instances.add(*i))
		}
	}

	#[inline(never)]
	fn init_query(&mut self, query: EntityQuery) {
		let data = crate::entities::get_query_data(query);

		// Match query against all archetypes
		let indices = self.vec.iter().enumerate().filter_map(|(i, a)| {
			if !a.matches_query(data.include()) {
				return None;
			}
			if a.matches_query(data.exclude()) {
				return None;
			}
			Some(i)
		});

		self.queries.insert(query, indices.collect());
	}
}
