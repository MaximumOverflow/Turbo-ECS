/// A unique handle to an `Entity`
#[derive(Clone, Debug)]
pub struct Entity {
	pub(crate) version: u32,
	pub(crate) registry_id: u32,
	pub(crate) instance: *mut EntityInstance,
}

pub(crate) struct EntityInstance {
	pub(crate) slot: usize,
	pub(crate) version: u32,
	pub(crate) archetype: usize,
}

impl Default for Entity {
	fn default() -> Self {
		Self {
			version: 0,
			registry_id: 0,
			instance: std::ptr::null_mut(),
		}
	}
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

impl Entity {
	#[inline(always)]
	pub(crate) fn get_instance(&self, context_id: u32) -> &EntityInstance {
		assert_entity(self, context_id);
		unsafe { &*self.instance }
	}

	#[inline(always)]
	pub(crate) fn get_instance_mut(&mut self, context_id: u32) -> &mut EntityInstance {
		assert_entity(self, context_id);
		unsafe { &mut *self.instance }
	}
}

#[inline(always)]
pub(crate) fn assert_entity(entity: &Entity, context_id: u32) {
	// SAFETY:
	// The entity's registry_id must be valid for the instance pointer to be de-referenced,
	// meaning the pointer is also still valid.
	unsafe {
		assert_eq!(entity.registry_id, context_id, "Entity does not belong to this context");
		assert_eq!(
			entity.version,
			(*entity.instance).version,
			"Entity has already been destroyed"
		);
	}
}
