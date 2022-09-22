use crate::archetypes::{Archetype, ArchetypeInstance};
use std::hash::{BuildHasherDefault, Hash};
use crate::data_structures::BitField;
use crate::components::ComponentType;
use crate::entities::EntityQuery;
use nohash_hasher::NoHashHasher;
use std::collections::HashMap;

type Hasher = BuildHasherDefault<NoHashHasher<usize>>;

pub(crate) struct ArchetypeStore {
	bf: BitField,
	vec: Vec<ArchetypeInstance>,
	map: HashMap<BitField, Archetype>,
	queries: HashMap<EntityQuery, Vec<usize>, Hasher>,
	transitions: HashMap<ArchetypeTransition, Archetype, Hasher>,
}

#[derive(Clone)]
pub(crate) struct ArchetypeTransition {
	pub archetype: Archetype,
	pub component: ComponentType,
	pub kind: ArchetypeTransitionKind,
}

#[repr(usize)]
#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub(crate) enum ArchetypeTransitionKind {
	Add = 0,
	Remove = 1,
}

impl ArchetypeStore {
	pub fn new() -> Self {
		Self {
			bf: BitField::new(),
			queries: HashMap::default(),
			map: HashMap::from([(BitField::new(), Archetype::default())]),
			vec: vec![ArchetypeInstance::new(Archetype { index: 0 }, &[])],
			transitions: HashMap::default(),
		}
	}

	/// Creates an [archetype](crate::archetypes::Archetype) containing the specified [components](crate::components::Component).
	pub fn create_archetype(&mut self, components: &[ComponentType]) -> Archetype {
		self.create_archetype_with_capacity(components, 0)
	}

	/// Creates an [archetype](crate::archetypes::Archetype) containing the specified [components](crate::components::Component) with the specified capacity.
	#[inline(never)]
	pub fn create_archetype_with_capacity(&mut self, components: &[ComponentType], min_capacity: usize) -> Archetype {
		let bitfield = &mut self.bf;
		bitfield.clear();

		for t in components {
			bitfield.set(t.id().value(), true);
		}

		if let Some(archetype) = self.map.get(bitfield) {
			self.vec[archetype.index as usize].ensure_capacity(min_capacity);
			return *archetype;
		}

		let archetype = Archetype { index: self.vec.len() };
		let instance = ArchetypeInstance::with_capacity(archetype, components, min_capacity);

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

		self.map.insert(bitfield.clone(), archetype);
		self.vec.push(instance);
		archetype
	}

	pub fn get(&self, index: usize) -> &ArchetypeInstance {
		&self.vec[index]
	}

	pub fn get_mut(&mut self, index: usize) -> &mut ArchetypeInstance {
		&mut self.vec[index]
	}

	pub fn query(&mut self, query: EntityQuery) -> impl Iterator<Item = &mut ArchetypeInstance> {
		if !self.queries.contains_key(&query) {
			self.init_query(query);
		}

		unsafe {
			let instances = self.vec.as_mut_ptr();
			self.queries.get(&query).unwrap().iter().map(move |i| &mut *instances.add(*i))
		}
	}

	pub fn get_archetype_transition(
		&mut self, transition: ArchetypeTransition,
	) -> Option<(&mut ArchetypeInstance, &mut ArchetypeInstance)> {
		fn get_refs(
			instances: &mut [ArchetypeInstance], src: Archetype, dst: Archetype,
		) -> (&mut ArchetypeInstance, &mut ArchetypeInstance) {
			unsafe {
				let src = &mut *(&mut instances[src.index] as *mut ArchetypeInstance);
				let dst = &mut *(&mut instances[dst.index] as *mut ArchetypeInstance);
				(src, dst)
			}
		}

		match self.transitions.get(&transition) {
			Some(archetype) => Some(get_refs(&mut self.vec, transition.archetype, *archetype)),

			None => match transition.kind {
				ArchetypeTransitionKind::Add => {
					let src = &self.vec[transition.archetype.index];
					if src.component_bitfield().get(transition.component.id().value()) {
						return None;
					}

					let bitfield = &mut self.bf;
					bitfield.copy_from(src.component_bitfield());
					bitfield.set(transition.component.id().value(), true);

					match self.map.get(bitfield) {
						Some(archetype) => Some(get_refs(&mut self.vec, transition.archetype, *archetype)),

						None => {
							let mut components = Vec::with_capacity(src.components().len() + 1);
							components.extend_from_slice(src.components());
							components.push(transition.component.clone());

							let archetype = self.create_archetype(&components);
							self.transitions.insert(transition.clone(), archetype);

							Some(get_refs(&mut self.vec, transition.archetype, archetype))
						},
					}
				},

				ArchetypeTransitionKind::Remove => {
					let src = &self.vec[transition.archetype.index];
					if !src.component_bitfield().get(transition.component.id().value()) {
						return None;
					}

					let bitfield = &mut self.bf;
					bitfield.copy_from(src.component_bitfield());
					bitfield.set(transition.component.id().value(), false);

					match self.map.get(bitfield) {
						Some(archetype) => Some(get_refs(&mut self.vec, transition.archetype, *archetype)),

						None => {
							let mut components = Vec::from(src.components());
							components
								.remove(components.iter().position(|t| t.id() == transition.component.id()).unwrap());

							let archetype = self.create_archetype(&components);
							self.transitions.insert(transition.clone(), archetype);

							Some(get_refs(&mut self.vec, transition.archetype, archetype))
						},
					}
				},
			},
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

impl Eq for ArchetypeTransition {}

impl PartialEq<Self> for ArchetypeTransition {
	fn eq(&self, other: &Self) -> bool {
		(self.component == other.component) & (self.archetype == other.archetype) & (self.kind == other.kind)
	}
}

impl Hash for ArchetypeTransition {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		let kind = self.kind as usize;
		let archetype = self.archetype.index << 33;
		let component = self.component.id().value() << 1;
		state.write_usize(kind | archetype | component);
	}
}
