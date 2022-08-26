#[macro_export]
macro_rules! create_archetype {
    ($ecs: expr, [$($t: ty),*]) => {
		$ecs.create_archetype(&[
			$(turbo_ecs::components::ComponentType::of::<$t>()),*
		])
	};
}