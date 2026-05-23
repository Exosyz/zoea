use crate::error::EcsError;
use zoea_core::ecs::entity::EntityId;

const DEFAULT_CAPACITY: usize = 1_000;

/// The centralized allocator responsible for managing entity lifecycles, recycling slots, and validating handle vitality.
///
/// `EntityManager` provides safe, deterministic creation and destruction of entities using a data-oriented
/// **Generational Index Allocation** layout. Dead slots are actively reclaimed into a LIFO (Last-In, First-Out)
/// freelist stack to guarantee that newly spawned entities maintain high CPU cache locality by tightly packing
/// live memory vectors.
///
/// ### Architecture Constraints
/// - Maximum simultaneous entity allocations: $2^{32} - 1$ (limited by the index size).
/// - Maximum times an individual index slot can be safely recycled: $2^{32} - 1$ before generation wrap-around.
pub struct EntityIdManager {
    /// Array mapping each index slot to its current tracking generation value.
    generations: Vec<u32>,
    /// A stack tracking the internal array indices of destroyed entities ready for memory recycling.
    free_slots: Vec<u32>,
}

impl EntityIdManager {
    /// Allocates an `EntityManager` context, pre-allocating heap layouts to avoid resize stutter during startup.
    ///
    /// Initial capacities default to `1_000` elements to minimize memory fragmentation during the engine's
    /// initial scene loading phase.
    pub fn new() -> Self {
        Self {
            generations: Vec::with_capacity(DEFAULT_CAPACITY),
            free_slots: Vec::with_capacity(DEFAULT_CAPACITY),
        }
    }

    /// Spawns a new entity into the registry context and returns its unique packed handle.
    ///
    /// ### Allocation Strategy
    /// 1. **Freelist Recycling (Fast Path):** If an index resides within the internal freelist, it is popped
    ///    immediately. The handle is created using this recycled index combined with its *retained, bumped* generation count.
    /// 2. **Vector Extension (Slow Path):** If no slots are free, the internal tracking array size grows by 1 row,
    ///    allocating a fresh index with a generation baseline of `0`.
    ///
    /// *Performance: Amortized $O(1)$. Popping from the freelist takes a constant few CPU cycles; extending the array is $O(1)$ unless a capacity resize occurs.*
    pub fn spawn(&mut self) -> EntityId {
        if let Some(index) = self.free_slots.pop() {
            let generation = self.generations[index as usize];
            EntityId::new(index, generation)
        } else {
            let index = self.generations.len() as u32;
            self.generations.push(0);
            EntityId::new(index, 0)
        }
    }

    /// Destroys an active entity, rendering all existing external handles pointing to it permanently invalid.
    ///
    /// ### Deallocation Strategy
    /// - Checks if the entity handle matches the current active generation state.
    /// - Uses standard wrapping arithmetic (`wrapping_add`) to advance the slot generation, preventing crashes if a slot is recycled $2^{32}$ times.
    /// - Appends the index to the internal tracking freelist to make it immediately available for the next call to [`Self::spawn`].
    ///
    /// ### Errors
    /// Returns `Err(EcsError::EntityAlreadyDead)` (or your designated dead entity error variant) if the provided
    /// handle fails safety verification checks, preventing double-kill bugs and memory corruption.
    ///
    /// *Performance: $O(1)$ execution time.*
    pub fn kill(&mut self, entity: EntityId) -> Result<(), EcsError> {
        let index = entity.index() as usize;

        if !self.is_alive(entity) {
            return Err(EcsError::EntityAlreadyDead);
        }

        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_slots.push(entity.index());

        Ok(())
    }

    /// Validates if an explicit `EntityId` handle points to a live entity with a matching allocation generation.
    ///
    /// This function acts as the core safety barrier within Zoea's query filters and system processing loops,
    /// ensuring that operations are never executed on data components belonging to long-deleted entities.
    ///
    /// ### Verification Pipeline
    /// 1. Asserts whether the handle index falls within the boundary bounds of the internal vector storage layout.
    /// 2. Asserts whether the target generation stored in memory perfectly matches the identifier payload generation.
    ///
    /// *Performance: $O(1)$ execution time. It boils down to a fast array boundary check and an integer comparison.*
    #[inline]
    pub fn is_alive(&self, entity: EntityId) -> bool {
        let index = entity.index() as usize;
        index < self.generations.len() && self.generations[index] == entity.generation()
    }
}

impl Default for EntityIdManager {
    /// Generates a default instantiation tracking context matching [`Self::new`].
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_packing_and_unpacking() {
        let index = 42u32;
        let generation = 7u32;
        let entity = EntityId::new(index, generation);

        assert_eq!(
            entity.index(),
            index,
            "Unpacked index values drifted from origin state"
        );
        assert_eq!(
            entity.generation(),
            generation,
            "Unpacked generation configuration drifted"
        );
    }

    #[test]
    fn test_entity_spawn_incremental_lifecycle() {
        let mut manager = EntityIdManager::new();

        let e1 = manager.spawn();
        let e2 = manager.spawn();

        assert_eq!(e1.index(), 0);
        assert_eq!(e1.generation(), 0);
        assert_eq!(e2.index(), 1);
        assert_eq!(e2.generation(), 0);

        assert!(manager.is_alive(e1));
        assert!(manager.is_alive(e2));
    }

    #[test]
    fn test_kill_and_freelist_recycling() {
        let mut manager = EntityIdManager::new();

        let e1 = manager.spawn();
        assert!(manager.is_alive(e1));

        // Kill entity
        let result = manager.kill(e1);
        assert!(result.is_ok());
        assert!(
            !manager.is_alive(e1),
            "Entity must be reported dead after explicit kill execution"
        );

        // Respawn should recycle index 0 but have an incremented generation (1)
        let e2 = manager.spawn();
        assert_eq!(
            e2.index(),
            0,
            "Freelist allocator failed to recycle index slot priority"
        );
        assert_eq!(
            e2.generation(),
            1,
            "Recycled index must advance generation to prevent ABA faults"
        );
        assert!(
            !manager.is_alive(e1),
            "Stale entity handles must remain invalidated"
        );
        assert!(manager.is_alive(e2));
    }

    #[test]
    fn test_double_kill_prevention() {
        let mut manager = EntityIdManager::new();
        let e1 = manager.spawn();

        assert!(manager.kill(e1).is_ok());

        // Second kill attempt must return error wrapper instead of double free mutation corruption
        let second_kill = manager.kill(e1);
        assert!(matches!(second_kill, Err(EcsError::EntityAlreadyDead)));
    }

    #[test]
    fn test_out_of_bounds_and_corrupted_id_safety() {
        let manager = EntityIdManager::new();

        // Generate an out-of-bounds fake ID manually
        let rogue_entity = EntityId::new(9999, 0);
        assert!(
            !manager.is_alive(rogue_entity),
            "Manager leaked positive validation for unallocated out-of-bound indexes"
        );
    }
}
