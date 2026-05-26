//! Central world orchestration engine managing entity lifecycles and structural transitions.
//!
//! The `World` acts as the primary container for all entity and component data, maintaining
//! the structural archetype graph network and routing entities across memory chunk boundaries
//! during dynamic runtime addition or removal operations.

use crate::entity::entity_builder::EntityBuilder;
use crate::entity::entity_id_manager::EntityIdManager;
use crate::entity::entity_location::EntityLocation;
use crate::error::EcsError;
use crate::storage::archetype::{Archetype, ArchetypeId};
use crate::storage::pending_component::PendingComponent;
use crate::topology::component_layout::ComponentLayout;
use crate::topology::component_mask::ComponentMask;
use crate::topology::component_registry::get_component_id;
use std::collections::HashMap;
use std::mem::forget;
use std::ptr::{read, NonNull};
use zoea_core::ecs::component::Component;
use zoea_core::ecs::entity::EntityId;

/// The central orchestrator of the Entity-Component-System (ECS).
///
/// The `World` is responsible for managing the lifecycle of all entities, keeping track
/// of their exact location in memory, and managing the `Archetype` graph.
/// It routes structural changes (adding/removing components) by moving entities
/// between dense SoA (Structure of Arrays) memory chunks.
#[derive(Default)]
pub struct World {
    /// Maps a unique signature of components to a specific Archetype.
    archetype_mask: HashMap<ComponentMask, ArchetypeId>,
    /// Stores the actual memory archetypes, indexed by their unique ID.
    archetype: HashMap<ArchetypeId, Archetype>,
    /// An incrementing counter to assign unique IDs to new Archetypes.
    next_archetype_id: usize,

    /// Maps a live `EntityId` to its exact memory location (Archetype, Chunk, and Index).
    locations: HashMap<EntityId, EntityLocation>,
    /// Manages the generation and recycling of `EntityId`s.
    entity_id_manager: EntityIdManager,
}

impl World {
    /// Creates a new, empty ECS `World`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initiates the creation of a new entity.
    ///
    /// Returns an [`EntityBuilder`] which allows chaining component additions
    /// before finally submitting the entity to the `World`'s memory storage.
    pub fn spawn(&'_ mut self) -> EntityBuilder<'_> {
        EntityBuilder::new(self)
    }

    /// Destroys an entity and entirely removes it from the ECS memory.
    ///
    /// This method uses a rapid `swap_remove` strategy to prevent memory fragmentation.
    /// The last entity in the affected memory chunk will be moved to fill the gap
    /// left by the killed entity, and the location registry is updated accordingly.
    ///
    /// # Errors
    /// Returns an `EcsError::EntityAlreadyDead` if the provided `EntityId` is invalid or already destroyed.
    pub fn kill(&mut self, id: EntityId) -> Result<(), EcsError> {
        let location = self.remove_entity_location(id)?;
        let archetype = self.get_archetype_mut(location.archetype_id)?;

        archetype.drop_entity_components(location.chunk_id, location.chunk_index);

        let moved_entity = archetype.swap_remove(location.chunk_id, location.chunk_index)?;

        // If another entity was moved to patch the hole left by the killed entity,
        // we must update its physical location mapping.
        if let Some(moved_id) = moved_entity {
            self.locations.insert(moved_id, location);
        }

        Ok(())
    }

    /// Dynamically adds a new component to an existing entity at runtime.
    ///
    /// This triggers a structural change: the entity's data is extracted from its
    /// current `Archetype` and moved into a new `Archetype` that matches its new
    /// component signature.
    ///
    /// This method utilizes a lazy graph edge cache. The first time a component type
    /// is added, the path is calculated via binary search and cached. Subsequent additions
    /// of the same component type hit the $O(1)$ cache fast-path.
    ///
    /// # Errors
    /// * `EcsError::EntityAlreadyDead` if the entity doesn't exist.
    /// * `EcsError::DuplicateComponent` if the entity already possesses this component type.
    pub fn add_component<T: Component>(
        &mut self,
        id: EntityId,
        component: T,
    ) -> Result<(), EcsError> {
        let old_location = self.get_entity_location(id)?;
        let added_component_id = get_component_id::<T>()?;

        let old_archetype = self.get_archetype_mut(old_location.archetype_id)?;
        let (new_archetype_id, new_layout_inserted_component_index) = match old_archetype
            .get_adding_edges(added_component_id)
        {
            Some(found_archetype) => found_archetype,
            None => {
                let mut old_layouts = old_archetype.layouts().clone();

                let (new_layout, new_layout_inserted_component_index) =
                    match old_layouts.binary_search_by_key(&added_component_id, |l| l.id) {
                        Ok(_) => return Err(EcsError::DuplicateComponent),
                        Err(not_found_index) => {
                            let component_layout = ComponentLayout::new::<T>(added_component_id);
                            old_layouts.insert(not_found_index, component_layout);
                            (old_layouts, not_found_index)
                        }
                    };

                let old_archetype_id = old_location.archetype_id;

                let mut new_mask = old_archetype.mask().clone();
                new_mask.insert(added_component_id);

                let new_archetype_id =
                    self.inner_get_or_create_archetype(new_mask, new_layout.clone())?;

                let old_archetype = self.get_archetype_mut(old_archetype_id)?;

                old_archetype.insert_adding_edges(
                    added_component_id,
                    new_archetype_id,
                    new_layout_inserted_component_index,
                );

                (new_archetype_id, new_layout_inserted_component_index)
            }
        };

        // SAFETY:
        // We create a raw pointer to an object currently on the stack.
        // This pointer is passed to the archetype's storage which will copy the bits
        // into the managed SoA memory, effectively "moving" the data out of the stack's control.
        let component_ptr = NonNull::new(&component as *const T as *mut u8).unwrap();

        self.move_entity(
            old_location,
            new_archetype_id,
            Some(component_ptr),
            new_layout_inserted_component_index,
        )?;

        // SAFETY:
        // Crucial step: The component's memory has been bitwise-copied into the archetype's chunk.
        // We must prevent Rust from running the destructor on the original stack variable,
        // otherwise, it would result in a double-free or invalid memory access.
        forget(component);

        Ok(())
    }

    /// Dynamically removes a component from an existing entity at runtime.
    ///
    /// This moves the entity to a narrower `Archetype`. It also ensures the memory
    /// of the removed component is properly read out of the old chunk and safely dropped.
    ///
    /// Like addition, this utilizes a lazy graph edge cache to completely bypass layout
    /// recalculation and binary searches after the first structural transition of this type.
    ///
    /// # Errors
    /// * `EcsError::EntityAlreadyDead` if the entity doesn't exist.
    /// * `EcsError::ComponentNotFound` if the entity does not have the specified component.
    pub fn remove_component<T: Component>(&mut self, id: EntityId) -> Result<(), EcsError> {
        let old_location = self.get_entity_location(id)?;
        let old_archetype = self.get_archetype_mut(old_location.archetype_id)?;

        let removed_component_id = get_component_id::<T>()?;
        let old_archetype_id = old_location.archetype_id;

        let (new_archetype_id, old_layout_removed_component_index) =
            match old_archetype.get_removing_edges(removed_component_id) {
                Some(found_archetype) => found_archetype,
                None => {
                    let mut old_layouts = old_archetype.layouts().clone();

                    let (new_layout, old_layout_removed_component_index) =
                        match old_layouts.binary_search_by_key(&removed_component_id, |l| l.id) {
                            Ok(found_index) => {
                                old_layouts.remove(found_index);
                                (old_layouts, found_index)
                            }
                            Err(_) => return Err(EcsError::ComponentNotFound),
                        };

                    let mut new_mask = old_archetype.mask().clone();
                    new_mask.remove(removed_component_id);

                    let new_archetype_id =
                        self.inner_get_or_create_archetype(new_mask, new_layout.clone())?;

                    let old_archetype = self.get_archetype_mut(old_archetype_id)?;

                    old_archetype.insert_removing_edges(
                        removed_component_id,
                        new_archetype_id,
                        old_layout_removed_component_index,
                    );

                    (new_archetype_id, old_layout_removed_component_index)
                }
            };

        let old_archetype = self.get_archetype_mut(old_archetype_id)?;

        // SAFETY:
        // We extract the component from the chunk by reading the raw bytes.
        // This effectively transfers ownership from the ECS memory back to the Rust stack.
        // Once `_removed_component` goes out of scope, its Drop implementation will be called,
        // cleaning up the component properly.
        let _removed_component = {
            let chunk = old_archetype.get_chunk(old_location.chunk_id)?;
            let ptr = chunk
                .get_component_ptr(old_layout_removed_component_index, old_location.chunk_index)?;
            unsafe { read(ptr.as_ptr() as *const T) }
        };

        self.move_entity(
            old_location,
            new_archetype_id,
            None,
            old_layout_removed_component_index,
        )?;

        Ok(())
    }

    /// Internal logic performing the actual cross-archetype transfer of an entity.
    ///
    /// Extracts all component pointers from the source archetype, splices in a new component
    /// (or removes one), and injects the resulting array into the destination archetype.
    fn move_entity(
        &mut self,
        src_location: EntityLocation,
        dst_id: ArchetypeId,
        component: Option<NonNull<u8>>,
        component_index: usize,
    ) -> Result<(), EcsError> {
        let mut ptrs = {
            let src = self.get_archetype_mut(src_location.archetype_id)?;
            src.extract_entity(src_location.chunk_id, src_location.chunk_index)
        }?;

        // The first pointer in the extraction is always the EntityId. We offset by 1.
        let component_index = component_index + 1;
        if let Some(component) = component {
            ptrs.insert(component_index, component);
        } else {
            ptrs.remove(component_index);
        }

        let new_location = {
            let dst = self.get_archetype_mut(dst_id)?;
            dst.inject_entity(src_location.id, ptrs)
        }?;

        let src = self.get_archetype_mut(src_location.archetype_id)?;
        let moved_entity = src.swap_remove(src_location.chunk_id, src_location.chunk_index)?;

        // Update the location of the entity that was moved to fill the structural gap.
        if let Some(moved_id) = moved_entity {
            self.locations.insert(moved_id, src_location);
        }

        // Register the new location for the actively transferred entity.
        self.insert_entity_location(src_location.id, new_location);

        Ok(())
    }

    /// Spawns a new unique identifier for an entity.
    pub(crate) fn generate_entity_id(&mut self) -> EntityId {
        self.entity_id_manager.spawn()
    }

    /// Registers or updates the physical memory location of a given entity.
    pub(crate) fn insert_entity_location(&mut self, id: EntityId, location: EntityLocation) {
        self.locations.insert(id, location);
    }

    /// Retrieves the current structural location of an entity.
    fn get_entity_location(&self, id: EntityId) -> Result<EntityLocation, EcsError> {
        self.locations
            .get(&id)
            .copied()
            .ok_or(EcsError::EntityAlreadyDead)
    }

    /// Removes an entity's location entry from the registry, usually prior to deletion.
    fn remove_entity_location(&mut self, id: EntityId) -> Result<EntityLocation, EcsError> {
        self.locations
            .remove(&id)
            .ok_or(EcsError::EntityAlreadyDead)
    }

    /// Retrieves an `ArchetypeId` that matches the provided layout of pending components,
    /// creating a new Archetype if no matching signature exists.
    pub(crate) fn get_or_create_archetype(
        &mut self,
        components: &[PendingComponent],
    ) -> Result<ArchetypeId, EcsError> {
        let mut mask = ComponentMask::new();

        for component in components.iter() {
            mask.insert(component.id)
        }

        self.inner_get_or_create_archetype(
            mask,
            components.iter().map(ComponentLayout::from).collect(),
        )
    }

    /// Core method to resolve archetype signatures. Finds an existing Archetype by its mask
    /// or instantiates a new one if it's the first time this specific layout is seen.
    fn inner_get_or_create_archetype(
        &mut self,
        mask: ComponentMask,
        layouts: Vec<ComponentLayout>,
    ) -> Result<ArchetypeId, EcsError> {
        match self.archetype_mask.get(&mask) {
            Some(id) => Ok(*id),
            None => {
                let id = ArchetypeId(self.next_archetype_id);
                self.next_archetype_id += 1;
                self.archetype.insert(id, Archetype::new(id, mask, layouts));
                self.archetype_mask.insert(mask, id);
                Ok(id)
            }
        }
    }

    /// Exposes a mutable reference to an Archetype for entity spawning or extraction.
    pub(crate) fn get_archetype_mut(
        &mut self,
        id: ArchetypeId,
    ) -> Result<&mut Archetype, EcsError> {
        self.archetype
            .get_mut(&id)
            .ok_or(EcsError::UnknownArchetype)
    }

    /// Exposes an immutable reference to an Archetype for reading component data.
    fn get_archetype(&self, id: ArchetypeId) -> Result<&Archetype, EcsError> {
        self.archetype.get(&id).ok_or(EcsError::UnknownArchetype)
    }

    /// Retrieves an immutable reference to a specific component attached to an entity.
    ///
    /// # Errors
    /// * `EcsError::EntityAlreadyDead` if the entity doesn't exist.
    /// * `EcsError::ComponentNotFound` if the entity does not possess the requested component type.
    pub fn get_component<T: Component>(&self, id: EntityId) -> Result<&T, EcsError> {
        let location = self.get_entity_location(id)?;
        let component_id = get_component_id::<T>()?;
        let archetype = self.get_archetype(location.archetype_id)?;

        if !archetype.mask().contains(component_id) {
            return Err(EcsError::ComponentNotFound);
        }

        let column_index = archetype
            .layouts()
            .binary_search_by_key(&component_id, |l| l.id)
            .map_err(|_| EcsError::ComponentNotFound)?;

        let chunk = archetype.get_chunk(location.chunk_id)?;
        let ptr = chunk.get_component_ptr(column_index, location.chunk_index)?;

        // SAFETY:
        // The ECS invariant ensures that the retrieved pointer is valid and correctly
        // points to an initialized instance of type T. Since we only return an immutable
        // reference (&T) and we are borrowing 'self' immutably, we strictly obey
        // Rust's borrowing rules (aliasing/mutability).
        Ok(unsafe { &*(ptr.as_ptr() as *const T) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_world_add_and_get_component() {
        let mut world = World::new();
        let e1 = spawn_empty_entity(&mut world, 1);

        world.add_component(e1, Position { x: 10, y: 20 }).unwrap();

        let pos = world.get_component::<Position>(e1).unwrap();
        assert_eq!(*pos, Position { x: 10, y: 20 });
    }

    #[test]
    fn test_world_add_and_remove_component_moves_entity() {
        let mut world = World::new();
        let e1 = spawn_empty_entity(&mut world, 2);

        // --- ADD ---
        let loc_initial = world.get_entity_location(e1).unwrap();
        world.add_component(e1, Position { x: 1, y: 1 }).unwrap();
        let loc_after_add = world.get_entity_location(e1).unwrap();

        assert_ne!(
            loc_initial.archetype_id, loc_after_add.archetype_id,
            "Archetype should change after adding a component"
        );
        assert!(world.get_component::<Position>(e1).is_ok());

        // --- REMOVE ---
        world.remove_component::<Position>(e1).unwrap();
        let loc_after_remove = world.get_entity_location(e1).unwrap();

        assert_ne!(
            loc_after_add.archetype_id, loc_after_remove.archetype_id,
            "Archetype should change after removing a component"
        );
        assert_eq!(
            loc_initial.archetype_id, loc_after_remove.archetype_id,
            "Entity should return to its initial archetype"
        );
        assert!(matches!(
            world.get_component::<Position>(e1),
            Err(EcsError::ComponentNotFound)
        ));
    }

    #[test]
    fn test_world_drop_is_called_on_remove_component() {
        let mut world = World::new();
        let e1 = spawn_empty_entity(&mut world, 3);

        let counter = Arc::new(Mutex::new(0));
        let droppable = DroppableComponent {
            counter: Arc::clone(&counter),
        };

        world.add_component(e1, droppable).unwrap();

        world.remove_component::<DroppableComponent>(e1).unwrap();

        assert_eq!(
            *counter.lock().unwrap(),
            1,
            "DroppableComponent should have been dropped once"
        );
    }

    #[test]
    fn test_world_drop_is_called_on_kill() {
        let mut world = World::new();
        let e1 = spawn_empty_entity(&mut world, 4);

        let counter = Arc::new(Mutex::new(0));
        let droppable = DroppableComponent {
            counter: Arc::clone(&counter),
        };

        world.add_component(e1, droppable).unwrap();

        // Killing the entity MUST trigger the drop of all its components
        world.kill(e1).unwrap();

        assert_eq!(
            *counter.lock().unwrap(),
            1,
            "Components must be dropped when an entity is killed to prevent memory leaks"
        );
    }

    #[test]
    fn test_world_error_handling() {
        let mut world = World::new();
        let e1 = spawn_empty_entity(&mut world, 1);

        // Test duplicate addition error
        world.add_component(e1, Position { x: 0, y: 0 }).unwrap();
        let res = world.add_component(e1, Position { x: 1, y: 1 });
        assert!(matches!(res, Err(EcsError::DuplicateComponent)));

        // Test actions on a dead entity
        world.kill(e1).unwrap();
        assert!(matches!(
            world.get_component::<Position>(e1),
            Err(EcsError::EntityAlreadyDead)
        ));
        assert!(matches!(world.kill(e1), Err(EcsError::EntityAlreadyDead)));
    }

    #[test]
    fn test_world_add_component_caches_graph_edges() {
        let mut world = World::new();
        let e1 = spawn_empty_entity(&mut world, 50);
        let loc_initial = world.get_entity_location(e1).unwrap();
        let comp_id = get_component_id::<Position>().unwrap();

        // 1. Assert the cache is completely empty initially
        {
            let arch = world.get_archetype(loc_initial.archetype_id).unwrap();
            assert!(
                arch.get_adding_edges(comp_id).is_none(),
                "Graph edge should not exist before the first addition"
            );
        }

        // 2. Trigger the first addition (Cache Miss -> Recalculate & Insert)
        world.add_component(e1, Position { x: 10, y: 20 }).unwrap();
        let loc_after = world.get_entity_location(e1).unwrap();

        // 3. Assert the edge was written back into the old archetype's cache
        {
            let arch = world.get_archetype(loc_initial.archetype_id).unwrap();
            let edge = arch.get_adding_edges(comp_id);

            assert!(
                edge.is_some(),
                "Graph edge was not saved to cache on a miss!"
            );
            let (target_arch_id, calculated_index) = edge.unwrap();

            assert_eq!(
                target_arch_id, loc_after.archetype_id,
                "Cached archetype mismatch"
            );
            assert_eq!(
                calculated_index, 0,
                "First component added should live at column layout index 0"
            );
        }

        // 4. Verify a second entity uses the cache seamlessly
        let e2 = spawn_empty_entity(&mut world, 51);
        world.add_component(e2, Position { x: 30, y: 40 }).unwrap(); // Cache Hit path execution

        let loc_e2_after = world.get_entity_location(e2).unwrap();
        assert_eq!(
            loc_e2_after.archetype_id, loc_after.archetype_id,
            "Entity 2 failed to route to the correct cached archetype node"
        );
    }

    #[test]
    fn test_world_remove_component_caches_graph_edges() {
        let mut world = World::new();
        let e1 = spawn_empty_entity(&mut world, 60);
        let comp_id = get_component_id::<Position>().unwrap();

        // Set up the entity with a component first
        world
            .add_component(e1, Position { x: 100, y: 100 })
            .unwrap();
        let loc_with_comp = world.get_entity_location(e1).unwrap();

        // 1. Assert the removal cache is empty
        {
            let arch = world.get_archetype(loc_with_comp.archetype_id).unwrap();
            assert!(arch.get_removing_edges(comp_id).is_none());
        }

        // 2. Trigger removal (Cache Miss -> Recalculate & Insert)
        world.remove_component::<Position>(e1).unwrap();
        let loc_after_remove = world.get_entity_location(e1).unwrap();

        // 3. Verify the shortcut edge was recorded correctly
        {
            let arch = world.get_archetype(loc_with_comp.archetype_id).unwrap();
            let edge = arch.get_removing_edges(comp_id);

            assert!(
                edge.is_some(),
                "Removal graph edge was not saved to cache on a miss!"
            );
            let (target_arch_id, removed_index) = edge.unwrap();

            assert_eq!(
                target_arch_id, loc_after_remove.archetype_id,
                "Cached fallback archetype mismatch"
            );
            assert_eq!(
                removed_index, 0,
                "The removed column index should match the target deletion index"
            );
        }
    }
}
