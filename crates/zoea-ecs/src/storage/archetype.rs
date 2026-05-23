use crate::entity::entity_location::EntityLocation;
use crate::error::EcsError;
use crate::storage::chunk::{Chunk, ChunkId};
use crate::storage::pending_component::PendingComponent;
use crate::topology::component_layout::ComponentLayout;
use crate::topology::component_mask::ComponentMask;
use crate::topology::component_registry::ComponentId;
use zoea_core::ecs::entity::EntityId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArchetypeId(pub usize);

pub struct Archetype {
    id: ArchetypeId,
    mask: ComponentMask,
    chunks: Vec<Chunk>,
    columns: Vec<ComponentLayout>,
    available_chunk_hint: usize,
}

impl Archetype {
    /// Creates a new structural Archetype layout based on a sample array of pending components.
    pub fn new(id: ArchetypeId, mask: ComponentMask, components: &[PendingComponent]) -> Self {
        Self {
            id,
            mask,
            chunks: Vec::with_capacity(1),
            columns: components.iter().map(ComponentLayout::from).collect(),
            available_chunk_hint: 0,
        }
    }

    /// Finds the localized column offset mapping index corresponding to a specific global `ComponentId`.
    #[inline]
    pub fn get_column_index(&self, id: ComponentId) -> Option<usize> {
        self.columns.iter().position(|layout| layout.id == id)
    }

    /// Spawns an entity into an available memory partition slot inside this archetype.
    ///
    /// # Errors
    /// Returns `EcsError::ComponentLimitExceeded` if the signature mapping density does not perfectly match column parameters.
    pub fn spawn(
        &mut self,
        entity_id: EntityId,
        components: &[PendingComponent],
    ) -> Result<EntityLocation, EcsError> {
        if components.len() != self.columns.len() {
            return Err(EcsError::ComponentLimitExceeded);
        }

        // Catch component layout mismatches in development before they corrupt raw SoA memory
        #[cfg(debug_assertions)]
        for (i, comp) in components.iter().enumerate() {
            // Assuming PendingComponent exposes its ComponentId. Adjust field name as needed.
            debug_assert_eq!(
                comp.id, self.columns[i].id,
                "Component mapping mismatch during spawn. Archetype expects {:?}, got {:?}",
                self.columns[i].id, comp.id
            );
        }

        let chunk_id = self.get_available_chunk_id()?;
        let chunk = self.get_chunk_mut(chunk_id);

        let chunk_index = unsafe { chunk.push(entity_id, components) }?;

        // If the chunk we just wrote to is now completely full, advance the free-space hint.
        if chunk.is_full() && chunk_id.0 == self.available_chunk_hint {
            self.available_chunk_hint += 1;
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

        let swapped_entity = unsafe { chunk.swap_remove(chunk_index) }?;

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
            Chunk::new(&self.columns)
        }?;
        self.chunks.push(new_chunk);

        let new_id = self.chunks.len() - 1;
        self.available_chunk_hint = new_id;

        Ok(ChunkId(new_id))
    }

    /// Accessor utility targeting safe index navigation across archetype-managed chunks.
    #[inline]
    fn get_chunk_mut(&mut self, id: ChunkId) -> &mut Chunk {
        &mut self.chunks[id.0]
    }

    /// Read-only accessor targeting managed chunks.
    #[inline]
    pub fn get_chunk(&self, id: ChunkId) -> Option<&Chunk> {
        self.chunks.get(id.0)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    fn setup_mock_components() -> (ComponentMask, Vec<PendingComponent>) {
        let mask = ComponentMask::new();
        let comps = vec![
            create_component(Position::from(10)),
            create_component(Velocity::from(10.0)),
        ];
        (mask, comps)
    }

    fn filled_one_chunk(archetype: &mut Archetype, comps: &[PendingComponent]) {
        let target_chunk_id = archetype.available_chunk_hint;
        let mut step = 0;

        loop {
            if let Some(chunk) = archetype.chunks().get(target_chunk_id)
                && chunk.is_full()
            {
                break;
            }

            let unique_entity_id = target_chunk_id * 50_000 + step + 1;
            archetype
                .spawn(create_entity(unique_entity_id as u32), comps)
                .unwrap();

            step += 1;
        }
    }

    #[test]
    fn test_initial_spawn_allocates_first_chunk() {
        let (mask, comps) = setup_mock_components();
        let mut archetype = Archetype::new(ArchetypeId(0), mask, &comps);

        assert_eq!(archetype.chunks().len(), 0);

        let loc = archetype.spawn(create_entity(100), &comps).unwrap();

        assert_eq!(archetype.chunks().len(), 1);
        assert_eq!(loc.chunk_id.0, 0);
        assert_eq!(loc.chunk_index, 0);
        assert_eq!(archetype.available_chunk_hint, 0);
    }

    #[test]
    fn test_chunk_saturation_advances_allocation_hint() {
        let (mask, comps) = setup_mock_components();
        let mut archetype = Archetype::new(ArchetypeId(0), mask, &comps);

        filled_one_chunk(&mut archetype, &comps);

        // Assert Chunk 0 is full, but hint hasn't moved yet because it points to the written item
        assert!(archetype.chunks[0].is_full());

        // Spawning entity 3 forces allocation of Chunk 1
        let loc = archetype.spawn(create_entity(12), &comps).unwrap();

        assert_eq!(archetype.chunks().len(), 2);
        assert_eq!(loc.chunk_id.0, 1);
        assert_eq!(archetype.available_chunk_hint, 1);
    }

    #[test]
    fn test_swap_remove_rewinds_hint_to_plug_fragmentation() {
        let (mask, comps) = setup_mock_components();
        let mut archetype = Archetype::new(ArchetypeId(0), mask, &comps);

        filled_one_chunk(&mut archetype, &comps);
        filled_one_chunk(&mut archetype, &comps);

        // Hint advanced to 2 (since Chunk 0 and 1 are saturated)
        assert_eq!(archetype.available_chunk_hint, 2);

        // Induce fragmentation: clear out a structural slot in Chunk 0
        archetype.swap_remove(ChunkId(0), 1).unwrap();

        // High Performance Verification: Hint instantly snaps back to chunk 0 to fill holes!
        assert_eq!(archetype.available_chunk_hint, 0);

        // Next spawn fills the structural memory hole instead of appending to the end
        let plug_loc = archetype.spawn(create_entity(5), &comps).unwrap();
        assert_eq!(plug_loc.chunk_id.0, 0);
        assert_eq!(plug_loc.chunk_index, 681);
    }

    #[test]
    fn test_allocation_path_uses_new_from_metadata_cloning() {
        let (mask, comps) = setup_mock_components();
        let mut archetype = Archetype::new(ArchetypeId(0), mask, &comps);

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
    }
}
