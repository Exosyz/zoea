use crate::storage::archetype::ArchetypeId;
use crate::storage::chunk::ChunkId;
use zoea_core::ecs::entity::EntityId;

/// Represents the exact memory location of an entity's components within the ECS.
///
/// In an archetype-based ECS, entities are grouped by their component composition
/// (Archetype), divided into fixed-size memory blocks (Chunks), and placed at a
/// specific offset within that chunk (`chunk_index`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityLocation {
    /// The unique identifier of the entity this location points to.
    /// Note: If this struct is stored in a map keyed by `EntityId`,
    /// consider removing this field to save memory and improve cache locality.
    pub id: EntityId,

    /// The identifier of the Archetype (the specific combination of components)
    /// this entity currently belongs to.
    pub archetype_id: ArchetypeId,

    /// The identifier of the specific memory Chunk within the Archetype
    /// where this entity's data is stored.
    pub chunk_id: ChunkId,

    /// The exact index/offset within the Chunk's internal arrays where
    /// the entity's components are located.
    pub chunk_index: usize,
}

impl EntityLocation {
    /// Creates a new `EntityLocation`.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the entity.
    /// * `archetype_id` - The ID of the archetype the entity belongs to.
    /// * `chunk_id` - The ID of the memory chunk storing the entity's components.
    /// * `chunk_index` - The index offset within the chunk.
    #[inline]
    pub fn new(
        id: EntityId,
        archetype_id: ArchetypeId,
        chunk_id: ChunkId,
        chunk_index: usize,
    ) -> Self {
        Self {
            id,
            archetype_id,
            chunk_id,
            chunk_index,
        }
    }
}
