use crate::archetypes::ArchetypeStore;
use crate::entities::EntityStore;
use std::collections::HashSet;
use crate::systems::System;
use std::any::TypeId;

pub(crate) struct SystemStore {
	state: State,
	set: HashSet<TypeId>,
	systems: Vec<Box<dyn System>>,
}

#[derive(Default)]
enum State {
	#[default]
	Uninitialized,
	Initializing,
	Initialized,
}

impl SystemStore {
	pub fn new() -> Self {
		Self {
			set: HashSet::default(),
			state: State::default(),
			systems: Vec::default(),
		}
	}

	pub fn add_system<T: 'static + System>(&mut self, system: T) {
		match self.state {
			State::Uninitialized => {
				let inserted = self.set.insert(TypeId::of::<T>());
				assert!(inserted, "System was already added to the current context");
				self.systems.push(Box::new(system));
			},
			State::Initializing => {
				panic!("Cannot add new systems during initialization");
			},
			State::Initialized => {
				panic!("Cannot add new systems after initialization");
			},
		}
	}

	pub fn setup_systems(&mut self, archetypes: &mut ArchetypeStore) {
		match self.state {
			State::Uninitialized => {
				self.state = State::Initializing;
				self.systems.iter_mut().for_each(|s| s.setup(archetypes));
				self.state = State::Initialized;
			},
			State::Initializing => {
				panic!("Recursive setup call to setup_systems")
			},
			State::Initialized => {
				panic!("Systems have already been initialized");
			},
		}
	}

	pub fn run_systems(&mut self, entities: &mut EntityStore) {
		match self.state {
			State::Uninitialized | State::Initializing => {
				panic!("Systems must be initialized before they can run");
			},
			State::Initialized => {
				self.systems.iter_mut().for_each(|s| s.run(entities));
			},
		}
	}
}
