use std::iter::repeat;

/// A unique handle to an `Entity`
#[derive(Default, Clone, Debug)]
pub struct Entity {
	pub(crate) index: u32,
	pub(crate) version: u32,
}

pub(crate) struct EntityInstance {
	pub(crate) slot: u32,
	pub(crate) version: u32,
	pub(crate) archetype: u32,
}

#[derive(Default)]
pub(crate) struct EntityInstanceVec {
	pub(crate) slots: Vec<u32>,
	pub(crate) versions: Vec<u32>,
	pub(crate) archetypes: Vec<u32>,
}

pub(crate) struct EntityInstanceRef<'l> {
	pub(crate) slot: &'l mut u32,
	pub(crate) version: &'l mut u32,
	pub(crate) archetype: &'l mut u32,
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

impl EntityInstanceVec {
	pub fn get(&self, index: usize) -> EntityInstance {
		EntityInstance {
			slot: self.slots[index],
			version: self.versions[index],
			archetype: self.archetypes[index],
		}
	}

	pub fn get_mut(&mut self, index: usize) -> EntityInstanceRef {
		EntityInstanceRef {
			slot: &mut self.slots[index],
			version: &mut self.versions[index],
			archetype: &mut self.archetypes[index]
		}
	}

	pub fn set(&mut self, index: usize, value: EntityInstance) {
		self.slots[index] = value.slot;
		self.versions[index] = value.version;
		self.archetypes[index] = value.archetype;
	}

	pub fn ensure_capacity(&mut self, capacity: usize) {
		if self.slots.len() < capacity {
			self.reserve(capacity - self.slots.len());
		}
	}

	pub fn reserve(&mut self, count: usize) {
		self.slots.extend(repeat(0).take(count));
		self.versions.extend(repeat(1).take(count));
		self.archetypes.extend(repeat(0).take(count));
	}
}

impl EntityInstanceRef<'_> {
	pub fn as_instance(&self) -> EntityInstance {
		EntityInstance {
			slot: *self.slot,
			version: *self.version,
			archetype: *self.archetype,
		}
	}
}

#[inline(always)]
pub(crate) fn assert_entity(entity: &Entity, instance: &EntityInstance) {
	#[cfg(not(feature = "debug_only_assertions"))]
	assert_eq!(
		entity.version, instance.version,
		"Entity has already been destroyed"
	);

	#[cfg(feature = "debug_only_assertions")]
	debug_assert_eq!(
		entity.version, instance.version,
		"Entity has already been destroyed"
	);
}

#[inline(always)]
pub(crate) fn assert_entity_version(entity: u32, instance: u32) {
	#[cfg(not(feature = "debug_only_assertions"))]
	assert_eq!(
		entity, instance,
		"Entity has already been destroyed"
	);

	#[cfg(feature = "debug_only_assertions")]
	debug_assert_eq!(
		entity, instance,
		"Entity has already been destroyed"
	);
}
