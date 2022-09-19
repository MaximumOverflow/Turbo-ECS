/// A unique handle to an `Entity`
#[derive(Default, Clone, Debug)]
pub struct Entity {
	pub(crate) index: u32,
	pub(crate) version: u16,
}

pub(crate) struct EntityInstance {
	pub(crate) slot: u32,
	pub(crate) version: u16,
	pub(crate) archetype: u16,
}

impl Default for EntityInstance {
	fn default() -> Self {
		Self {
			slot: 0,
			version: 1,
			archetype: 0,
		}
	}
}

#[inline(always)]
pub(crate) fn assert_entity(entity: &Entity, instance: &EntityInstance) {
	#[cfg(not(feature = "debug_only_assertions"))]
	assert_eq!(entity.version, instance.version, "Entity has already been destroyed");

	#[cfg(feature = "debug_only_assertions")]
	debug_assert_eq!(entity.version, instance.version, "Entity has already been destroyed");
}
