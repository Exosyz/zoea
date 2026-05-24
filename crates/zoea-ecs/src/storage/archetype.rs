//! Structural archetype containers organizing heterogeneous collections of dense data chunks.
//!
//! An Archetype uniquely represents a single specific combination of components. It manages
//! a graph-like network of structural edges to optimize components addition or removal transitions,
//! alongside a continuous vector of data chunks that share identical layouts.

use crate::entity::entity_location::EntityLocation;
use crate::error::EcsError;
use crate::storage::chunk::{Chunk, ChunkId};
use crate::storage::pending_component::PendingComponent;
use crate::topology::component_layout::ComponentLayout;
use crate::topology::component_mask::ComponentMask;
use crate::topology::component_registry::ComponentId;
use std::collections::HashMap;
use std::ptr::NonNull;
use zoea_core::ecs::entity::EntityId;

/// Unique tracking identifier assigned to an individual structural Archetype combination.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArchetypeId(pub usize);

/// Container grouping entities that share identical component configurations.
///
/// Manages continuous execution chunks and maintains direct routing references
/// to maximize structural migration speeds across runtime frame updates.
pub struct Archetype {
    id: ArchetypeId,
    mask: ComponentMask,
    chunks: Vec<Chunk>,
    layouts: Vec<ComponentLayout>,
    available_chunk_hint: usize,

    /// Cached edges for O(1) Archetype graph routing when adding components.
    pub add_edges: HashMap<ComponentId, ArchetypeId>,
    /// Cached edges for O(1) Archetype graph routing when removing components.
    pub remove_edges: HashMap<ComponentId, ArchetypeId>,
}

impl Archetype {
    /// Creates a new structural Archetype layout based on component descriptions.
    pub fn new(id: ArchetypeId, mask: ComponentMask, layouts: Vec<ComponentLayout>) -> Self {
        Self {
            id,
            mask,
            chunks: Vec::with_capacity(1),
            layouts,
            available_chunk_hint: 0,
            add_edges: HashMap::new(),
            remove_edges: HashMap::new(),
        }
    }

    /// Finds the localized column offset mapping index corresponding to a specific global `ComponentId`.
    #[inline]
    pub fn get_column_index(&self, id: ComponentId) -> Option<usize> {
        self.layouts.iter().position(|layout| layout.id == id)
    }

    /// Spawns an entity into an available memory partition slot inside this archetype.
    ///
    /// # Errors
    /// Returns `EcsError::ComponentLimitExceeded` if the signature mapping density does not perfectly match column parameters.
    pub fn spawn(
        &mut self,
        entity_id: EntityId,
        components: Vec<PendingComponent>,
    ) -> Result<EntityLocation, EcsError> {
        if components.len() != self.layouts.len() {
            return Err(EcsError::ComponentLimitExceeded);
        }

        // Catch component layout mismatches in development before they corrupt raw SoA memory
        #[cfg(debug_assertions)]
        for (i, comp) in components.iter().enumerate() {
            debug_assert_eq!(
                comp.id, self.layouts[i].id,
                "Component mapping mismatch during spawn. Archetype expects {:?}, got {:?}",
                self.layouts[i].id, comp.id
            );
        }

        let chunk_id = self.get_available_chunk_id()?;

        let raw_ptrs: Vec<NonNull<u8>> = components.iter().map(|c| c.ptr).collect();

        let chunk = self.get_chunk_mut(chunk_id);
        let chunk_index = unsafe { chunk.push(entity_id, &raw_ptrs) }?;

        // If the chunk we just wrote to is now completely full, advance the free-space hint.
        if chunk.is_full() && chunk_id.0 == self.available_chunk_hint {
            self.available_chunk_hint += 1;
        }

        for comp in components {
            unsafe {
                comp.release_allocation_shell();
            }
        }

        let location = EntityLocation::new(entity_id, self.id, chunk_id, chunk_index);
        Ok(location)
    }

    /// Removes an entity layout via rapid swap-remove mechanics from a target dense chunk array slot.
    /// Returns the moved `EntityId` that patched the structural gap, if one occurred.
    pub fn swap_remove(
        &mut self,
        chunk_id: ChunkId,
        chunk_index: usize,
    ) -> Result<Option<EntityId>, EcsError> {
        let chunk = self.get_chunk_mut(chunk_id);
        let swapped_entity = unsafe { chunk.swap_remove_and_forget(chunk_index) }?;

        // We just freed a slot via swap_remove.
        // Pull the hint backwards so the next spawn immediately fills this memory fragmentation.
        if chunk_id.0 < self.available_chunk_hint {
            self.available_chunk_hint = chunk_id.0;
        }

        Ok(swapped_entity)
    }

    /// Scans existing architectural allocations for a vacant storage chunk or instantiates a fresh twin block.
    fn get_available_chunk_id(&mut self) -> Result<ChunkId, EcsError> {
        if self.available_chunk_hint < self.chunks.len() {
            let chunk = &self.chunks[self.available_chunk_hint];
            if !chunk.is_full() {
                return Ok(ChunkId(self.available_chunk_hint));
            } else {
                // Defensive fallback: If the hint somehow desynced, scan forward from the hint
                for (index, chunk) in self
                    .chunks
                    .iter()
                    .enumerate()
                    .skip(self.available_chunk_hint)
                {
                    if !chunk.is_full() {
                        self.available_chunk_hint = index;
                        return Ok(ChunkId(index));
                    }
                }
            }
        }

        let new_chunk = if let Some(first_chunk) = self.chunks.first() {
            Chunk::new_from(first_chunk)
        } else {
            Chunk::new(&self.layouts)
        }?;
        self.chunks.push(new_chunk);

        let new_id = self.chunks.len() - 1;
        self.available_chunk_hint = new_id;

        Ok(ChunkId(new_id))
    }

    /// Extracts all structural column component data pointers belonging to a row location index.
    pub fn extract_entity(
        &mut self,
        chunk_id: ChunkId,
        chunk_index: usize,
    ) -> Result<Vec<NonNull<u8>>, EcsError> {
        let chunk = self.get_chunk_mut(chunk_id);
        unsafe { chunk.extract_entity(chunk_index) }
    }

    /// Injects raw pointer data sequences straight into layout aligned chunk arrays.
    pub fn inject_entity(
        &mut self,
        entity_id: EntityId,
        ptrs: Vec<NonNull<u8>>,
    ) -> Result<EntityLocation, EcsError> {
        let chunk_id = self.get_available_chunk_id()?;
        let chunk = self.get_chunk_mut(chunk_id);

        let chunk_index = unsafe { chunk.inject_entity(ptrs)? };
        Ok(EntityLocation::new(
            entity_id,
            self.id,
            chunk_id,
            chunk_index,
        ))
    }

    /// Safely executes the `Drop` implementation for all components attached to an entity
    /// located at the specified chunk and index.
    ///
    /// # Safety
    /// This method must only be called when an entity is being permanently destroyed
    /// (e.g., during `kill`) or when the memory slot is guaranteed to be invalidated
    /// immediately after this call. Accessing these components after this method
    /// returns is undefined behavior.
    pub fn drop_entity_components(&mut self, chunk_id: ChunkId, chunk_index: usize) {
        let layouts = self.layouts.to_vec();
        let chunk = self.get_chunk_mut(chunk_id);

        for (col_idx, layout) in layouts.iter().enumerate() {
            if let Ok(ptr) = chunk.get_component_ptr(col_idx, chunk_index) {
                // SAFETY: The component layout is guaranteed to provide the correct
                // drop function pointer for the type stored in this column.
                unsafe { (layout.drop_fn)(ptr) };
            }
        }
    }

    /// Accessor utility targeting safe index navigation across archetype-managed chunks.
    #[inline]
    fn get_chunk_mut(&mut self, id: ChunkId) -> &mut Chunk {
        &mut self.chunks[id.0]
    }

    /// Read-only accessor targeting managed chunks.
    #[inline]
    pub fn get_chunk(&self, id: ChunkId) -> Result<&Chunk, EcsError> {
        self.chunks.get(id.0).ok_or(EcsError::ComponentNotFound)
    }

    /// Exposes read-only access to the archetype's structural chunk layout configurations.
    #[inline]
    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    /// Returns the unique `ArchetypeId` tracking identifier.
    #[inline]
    pub fn id(&self) -> ArchetypeId {
        self.id
    }

    /// Exposes the operational `ComponentMask` bit-field signature defining this archetype.
    #[inline]
    pub fn mask(&self) -> &ComponentMask {
        &self.mask
    }

    /// Exposes read-only layout sequence listings matching physical column structures.
    #[inline]
    pub fn layouts(&self) -> &Vec<ComponentLayout> {
        &self.layouts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use std::sync::{Arc, Mutex};

    fn setup_mock_components() -> (ComponentMask, Vec<PendingComponent>) {
        let mask = ComponentMask::new();
        let comps = vec![
            create_component(Position::from(10)),
            create_component(Velocity::from(10.0)),
        ];
        (mask, comps)
    }

    /// Safely floods an entire chunk configuration by utilizing our safe deep test cloning hook.
    fn filled_one_chunk(archetype: &mut Archetype, blueprints: &[PendingComponent]) {
        let target_chunk_id = archetype.available_chunk_hint;
        let mut step = 0;

        loop {
            if let Some(chunk) = archetype.chunks().get(target_chunk_id)
                && chunk.is_full()
            {
                break;
            }

            let unique_entity_id = target_chunk_id * 50_000 + step + 1;

            // Deep clone components using our custom test hook to prevent raw pointer double-frees
            let cloned_comps = vec![
                blueprints[0].test_clone::<Position>(),
                blueprints[1].test_clone::<Velocity>(),
            ];

            archetype
                .spawn(create_entity(unique_entity_id as u32), cloned_comps)
                .unwrap();

            step += 1;
        }
    }

    #[test]
    fn test_initial_spawn_allocates_first_chunk() {
        let (mask, comps) = setup_mock_components();
        let mut archetype = Archetype::new(
            ArchetypeId(0),
            mask,
            comps.iter().map(ComponentLayout::from).collect(),
        );

        assert_eq!(archetype.chunks().len(), 0);

        let spawn_comps = vec![
            comps[0].test_clone::<Position>(),
            comps[1].test_clone::<Velocity>(),
        ];
        let loc = archetype.spawn(create_entity(100), spawn_comps).unwrap();

        assert_eq!(archetype.chunks().len(), 1);
        assert_eq!(loc.chunk_id.0, 0);
        assert_eq!(loc.chunk_index, 0);
        assert_eq!(archetype.available_chunk_hint, 0);

        drop(comps);
    }

    #[test]
    fn test_chunk_saturation_advances_allocation_hint() {
        let (mask, comps) = setup_mock_components();
        let mut archetype = Archetype::new(
            ArchetypeId(0),
            mask,
            comps.iter().map(ComponentLayout::from).collect(),
        );

        filled_one_chunk(&mut archetype, &comps);
        assert!(archetype.chunks[0].is_full());

        let spawn_comps = vec![
            comps[0].test_clone::<Position>(),
            comps[1].test_clone::<Velocity>(),
        ];
        let loc = archetype.spawn(create_entity(12), spawn_comps).unwrap();

        assert_eq!(archetype.chunks().len(), 2);
        assert_eq!(loc.chunk_id.0, 1);
        assert_eq!(archetype.available_chunk_hint, 1);

        drop(comps);
    }

    #[test]
    fn test_swap_remove_rewinds_hint_to_plug_fragmentation() {
        let (mask, comps) = setup_mock_components();
        let mut archetype = Archetype::new(
            ArchetypeId(0),
            mask,
            comps.iter().map(ComponentLayout::from).collect(),
        );

        filled_one_chunk(&mut archetype, &comps);
        filled_one_chunk(&mut archetype, &comps);

        assert_eq!(archetype.available_chunk_hint, 2);

        archetype.swap_remove(ChunkId(0), 1).unwrap();

        assert_eq!(archetype.available_chunk_hint, 0);

        let spawn_comps = vec![
            comps[0].test_clone::<Position>(),
            comps[1].test_clone::<Velocity>(),
        ];
        let plug_loc = archetype.spawn(create_entity(5), spawn_comps).unwrap();
        assert_eq!(plug_loc.chunk_id.0, 0);

        drop(comps);
    }

    #[test]
    fn test_allocation_path_uses_new_from_metadata_cloning() {
        let (mask, comps) = setup_mock_components();
        let mut archetype = Archetype::new(
            ArchetypeId(0),
            mask,
            comps.iter().map(ComponentLayout::from).collect(),
        );

        filled_one_chunk(&mut archetype, &comps);
        filled_one_chunk(&mut archetype, &comps);
        filled_one_chunk(&mut archetype, &comps);

        assert_eq!(archetype.chunks().len(), 3);

        let clone_count = archetype
            .chunks()
            .iter()
            .filter(|c| c.was_cloned_via_new_from)
            .count();

        assert_eq!(
            clone_count, 2,
            "Archetype failed to clone structural metadata using Chunk::new_from!"
        );

        assert!(!archetype.chunks[0].was_cloned_via_new_from);
        assert!(archetype.chunks[1].was_cloned_via_new_from);
        assert!(archetype.chunks[2].was_cloned_via_new_from);

        drop(comps);
    }

    #[test]
    fn test_archetype_inject_and_extract() {
        let mut mask = ComponentMask::new();
        mask.insert(get_id::<Position>());

        let layouts = vec![create_layout::<Position>(drop_noop_fn)];
        let archetype_id = ArchetypeId(1);

        let mut archetype = Archetype::new(archetype_id, mask, layouts);

        let mut entity_id = create_entity(99);
        let pos_comp = create_component(Position { x: 100, y: 200 });

        let ptrs_to_inject = vec![get_entity_id_ptr(&mut entity_id), pos_comp.ptr];

        let location = archetype
            .inject_entity(entity_id, ptrs_to_inject)
            .expect("Injection into archetype failed");

        assert_eq!(location.id, entity_id);
        assert_eq!(location.archetype_id, archetype_id);
        assert_eq!(location.chunk_index, 0);

        let extracted_ptrs = archetype
            .extract_entity(location.chunk_id, location.chunk_index)
            .expect("Extraction from archetype failed");

        assert_eq!(extracted_ptrs.len(), 2);

        unsafe {
            let extracted_id = *(extracted_ptrs[0].as_ptr() as *const EntityId);
            let extracted_pos = *(extracted_ptrs[1].as_ptr() as *const Position);

            assert_eq!(extracted_id, entity_id);
            assert_eq!(extracted_pos, Position { x: 100, y: 200 });

            pos_comp.release_allocation_shell();
        }
    }

    #[test]
    fn test_archetype_multiple_injections_fill_chunks() {
        let mut mask = ComponentMask::new();
        mask.insert(get_id::<Velocity>());

        let layouts = vec![create_layout::<Velocity>(drop_noop_fn)];
        let archetype_id = ArchetypeId(2);
        let mut archetype = Archetype::new(archetype_id, mask, layouts);

        let mut e1_id = create_entity(1);
        let vel1 = create_component(Velocity { dx: 1.0, dy: 1.0 });
        let loc1 = archetype
            .inject_entity(e1_id, vec![get_entity_id_ptr(&mut e1_id), vel1.ptr])
            .unwrap();

        let mut e2_id = create_entity(2);
        let vel2 = create_component(Velocity { dx: 2.0, dy: 2.0 });
        let loc2 = archetype
            .inject_entity(e2_id, vec![get_entity_id_ptr(&mut e2_id), vel2.ptr])
            .unwrap();

        assert_eq!(loc1.id, e1_id);
        assert_eq!(loc2.id, e2_id);

        unsafe {
            let ptrs_e2 = archetype
                .extract_entity(loc2.chunk_id, loc2.chunk_index)
                .unwrap();
            let extracted_vel2 = *(ptrs_e2[1].as_ptr() as *const Velocity);
            assert_eq!(extracted_vel2, Velocity { dx: 2.0, dy: 2.0 });

            vel1.release_allocation_shell();
            vel2.release_allocation_shell();
        }
    }

    #[test]
    fn test_archetype_drop_entity_components() {
        let mut mask = ComponentMask::new();
        mask.insert(get_id::<DroppableComponent>());

        let layout_drop =
            create_layout::<DroppableComponent>(drop_component_fn::<DroppableComponent>);
        let mut archetype = Archetype::new(ArchetypeId(1), mask, vec![layout_drop]);

        let drop_counter = Arc::new(Mutex::new(0));
        let tracker = DroppableComponent {
            counter: Arc::clone(&drop_counter),
        };
        let mut entity_id = create_entity(1);

        let pending = create_component(tracker);
        let ptrs = vec![
            NonNull::new(&mut entity_id as *mut _ as *mut u8).unwrap(),
            pending.ptr,
        ];

        let location = archetype.inject_entity(entity_id, ptrs).unwrap();

        archetype.drop_entity_components(location.chunk_id, location.chunk_index);

        assert_eq!(
            *drop_counter.lock().unwrap(),
            1,
            "The drop_fn should have been executed by drop_entity_components"
        );

        let _ = archetype.swap_remove(location.chunk_id, location.chunk_index);

        unsafe { pending.release_allocation_shell() };
    }
}
