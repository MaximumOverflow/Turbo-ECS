use crate::archetypes::ArchetypeStore;
use crate::entities::EntityRegistry;

pub trait System {
	#[allow(unused_variables)]
	fn setup(&mut self, archetypes: &mut ArchetypeStore) {}
	fn run(&mut self, entities: &mut EntityRegistry);
}
