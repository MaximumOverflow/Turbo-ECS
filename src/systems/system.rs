use crate::archetypes::ArchetypeStore;
use crate::entities::EntityStore;

pub trait System {
	#[allow(unused_variables)]
	fn setup(&mut self, archetypes: &mut ArchetypeStore) {}
	fn run(&mut self, entities: &mut EntityStore);
}
