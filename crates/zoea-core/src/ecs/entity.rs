/// A globally unique, bit-packed 64-bit handle representing an Entity in the Zoea ECS.
///
/// To optimize cache locality, memory usage, and cross-thread messaging overhead,
/// `EntityId` avoids pointer tracking and instead uses a packed layout split into two 32-bit registers:
///
/// ```text
///  64            32                             0
///  +-------------+------------------------------+
///  | Generation  |         Storage Index        |
///  +-------------+------------------------------+
/// ```
///
/// ### Internals & Safety
/// - **Storage Index (Lower 32 bits):** Directly maps to the entity's row offset within
///   component arrays and structural column storages.
/// - **Generation (Upper 32 bits):** A monotonically incrementing counter used to identify
///   and invalidate stale handles pointing to recycled index slots, completely eliminating the
///   **ABA problem** at the architectural level.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(u64);

impl EntityId {
    /// Packs a raw 32-bit index slot and a 32-bit validation generation into a single 64-bit `EntityId`.
    ///
    /// This function performs basic bitwise shifting and mask composition. It is designed to be
    /// zero-cost and completely transparent to the compiler for optimization pipelines.
    #[inline]
    pub const fn new(index: u32, generation: u32) -> Self {
        Self(((generation as u64) << 32) | (index as u64))
    }

    /// Extracts the raw tracking index associated with this entity.
    ///
    /// This index is safe to use for direct indexing into dense array backends or sparse-set pages
    /// after verifying structural vitality with [`EntityManager::is_alive`].
    ///
    /// *Performance: $O(1)$ — Compiles down to a zero-overhead truncation instruction on x86_64 and ARM.*
    #[inline]
    pub const fn index(self) -> u32 {
        self.0 as u32
    }

    /// Extracts the specific historical allocation generation of this handle.
    ///
    /// Used by internal runtime arrays to guarantee that a handle was created during the current
    /// life cycle of the underlying index allocation slot.
    ///
    /// *Performance: $O(1)$ — Compiles down to a single logical hardware bit-shift instruction.*
    #[inline]
    pub const fn generation(self) -> u32 {
        (self.0 >> 32) as u32
    }
}