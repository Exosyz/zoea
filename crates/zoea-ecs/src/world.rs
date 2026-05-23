use crate::entity::entity_builder::EntityBuilder;
use crate::entity::entity_id_manager::EntityIdManager;
use crate::entity::entity_location::EntityLocation;
use crate::error::EcsError;
use crate::storage::archetype::{Archetype, ArchetypeId};
use crate::storage::pending_component::PendingComponent;
use crate::topology::component_mask::ComponentMask;
use std::collections::HashMap;
use zoea_core::ecs::component::Component;
use zoea_core::ecs::entity::EntityId;
use zoea_core::rendering::assets::Sprite;
use zoea_core::transform::Transform;

pub struct TempEntity {
    pub transform: Transform,
    pub sprite: Sprite,
}
#[derive(Default)]
pub struct World {
    archetype_mask: HashMap<ComponentMask, ArchetypeId>,
    archetype: HashMap<ArchetypeId, Archetype>,
    next_archetype_id: usize,

    locations: HashMap<EntityId, EntityLocation>,
    entity_id_manager: EntityIdManager,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&'_ mut self) -> EntityBuilder<'_> {
        EntityBuilder::new(self)
    }

    pub fn kill(&mut self, id: EntityId) -> Result<(), EcsError> {
        let location = self
            .locations
            .remove(&id)
            .ok_or(EcsError::EntityAlreadyDead)?;
        let archetype = self.get_archetype_mut(location.archetype_id)?;

        let moved_entity = archetype.swap_remove(location.chunk_id, location.chunk_index)?;

        if let Some(moved_id) = moved_entity {
            self.locations.insert(moved_id, location);
        }

        Ok(())
    }

    pub fn add_component<T: Component>(&mut self, id: EntityId, component: T) {}

    pub fn remove_component<T: Component>(&mut self, id: EntityId) {}

    fn move_archetype(
        entity_id: EntityId,
        current_archetype: Archetype,
        target_archetype: Archetype,
    ) {
    }

    pub(crate) fn generate_entity_id(&mut self) -> EntityId {
        self.entity_id_manager.spawn()
    }

    pub(crate) fn set_entity_location(&mut self, id: EntityId, location: EntityLocation) {
        self.locations.insert(id, location);
    }

    pub(crate) fn get_entity_location(&self, id: EntityId) -> Result<&EntityLocation, EcsError> {
        self.locations.get(&id).ok_or(EcsError::EntityAlreadyDead)
    }

    pub(crate) fn get_or_create_archetype(
        &mut self,
        components: &[PendingComponent],
    ) -> Result<ArchetypeId, EcsError> {
        let mut mask = ComponentMask::new();

        for component in components.iter() {
            mask.insert(component.id)
        }

        match self.archetype_mask.get(&mask) {
            Some(id) => Ok(*id),
            None => {
                let id = ArchetypeId(self.next_archetype_id);
                self.next_archetype_id += 1;
                self.archetype
                    .insert(id, Archetype::new(id, mask, components));
                self.archetype_mask.insert(mask, id);
                Ok(id)
            }
        }
    }

    pub(crate) fn get_archetype_mut(
        &mut self,
        id: ArchetypeId,
    ) -> Result<&mut Archetype, EcsError> {
        self.archetype
            .get_mut(&id)
            .ok_or(EcsError::UnknownArchetype)
    }
}
