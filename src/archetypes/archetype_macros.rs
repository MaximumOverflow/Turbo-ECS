/// Create a new [Archetype](crate::archetypes::Archetype) in the specified [EcsContext](crate::context::EcsContext).
#[macro_export]
macro_rules! create_archetype {
    ($ecs: expr, [$($t: ty),*]) => {
		$ecs.create_archetype(&[
			$(turbo_ecs::components::ComponentType::of::<$t>()),*
		])
	};
}
