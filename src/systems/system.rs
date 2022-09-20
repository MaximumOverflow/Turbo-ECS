use crate::entities::EntityRegistry;

/// It provides the logic for modifying the state of [Entities](crate::entities::Entity)
/// and their associated [Components](crate::components::Component).
pub trait System {
	/// Initialises the [System].
	/// **This function should not be called by user code.**
	fn setup(&mut self) {}

	/// Executes the system
	fn run(&mut self, entities: &mut EntityRegistry);
}
