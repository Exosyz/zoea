use crate::topology::component_registry::ComponentId;

/// The maximum number of distinct components supported by the engine mask.
pub const MAX_COMPONENTS: usize = 256;

/// A 256-bit flat component tracking mask optimized for archetype query filters.
///
/// It distributes 256 structural bits across four 64-bit storage blocks (`u64`),
/// ensuring O(1) allocation-free evaluations during entity filtering.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ComponentMask {
    /// Four 64-bit storage blocks forming the total 256-bit bitmask array.
    blocks: [u64; 4],
}

impl ComponentMask {
    /// Creates a blank `ComponentMask` with all bits initialized to 0.
    #[inline]
    pub fn new() -> Self {
        Self { blocks: [0; 4] }
    }

    /// Sets the matching component identifier flag bit to active.
    #[inline]
    pub fn insert(&mut self, component_id: ComponentId) {
        let (block_idx, bit_idx) = Self::get_coords(component_id);
        self.blocks[block_idx] |= 1 << bit_idx;
    }

    /// Clears the matching component identifier flag a bit back to zero.
    #[inline]
    pub fn remove(&mut self, component_id: ComponentId) {
        let (block_idx, bit_idx) = Self::get_coords(component_id);
        self.blocks[block_idx] &= !(1 << bit_idx);
    }

    /// Confirms whether a specific component flag bit is active.
    #[inline]
    pub fn contains(&self, component_id: ComponentId) -> bool {
        let (block_idx, bit_idx) = Self::get_coords(component_id);
        self.blocks[block_idx] & (1 << bit_idx) != 0
    }

    /// Internal coordinate resolver translating a `ComponentId` to structural block coordinates.
    ///
    /// # Panics
    /// if the `ComponentId` violates bounds constraints (>= 256).
    #[inline]
    fn get_coords(component_id: ComponentId) -> (usize, usize) {
        let id = component_id.0;

        // Hard assertion guard. Out of bounds IDs imply deep structural engine failure.
        if id >= MAX_COMPONENTS {
            panic!(
                "ComponentMask boundary fault: ID {} exceeds limit of 256",
                id
            );
        }

        // Optimized using bitwise operations:
        // id >> 6 is equivalent to id / 64
        // id & 63 is equivalent to id % 64
        (id >> 6, id & 63)
    }

    /// Evaluates whether this mask contains **all** active flag states specified by the subset filter.
    #[inline]
    pub fn contains_all(&self, other: &ComponentMask) -> bool {
        (self.blocks[0] & other.blocks[0]) == other.blocks[0]
            && (self.blocks[1] & other.blocks[1]) == other.blocks[1]
            && (self.blocks[2] & other.blocks[2]) == other.blocks[2]
            && (self.blocks[3] & other.blocks[3]) == other.blocks[3]
    }

    /// Evaluates whether this mask shares **at least one** active flag state intersection with a filter.
    #[inline]
    pub fn contains_any(&self, other: &ComponentMask) -> bool {
        (self.blocks[0] & other.blocks[0]) != 0
            || (self.blocks[1] & other.blocks[1]) != 0
            || (self.blocks[2] & other.blocks[2]) != 0
            || (self.blocks[3] & other.blocks[3]) != 0
    }
}

impl Default for ComponentMask {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_initialization_state() {
        let mask = ComponentMask::new();
        assert_eq!(mask.blocks, [0; 4]);
    }

    #[test]
    fn test_mutations_across_block_boundaries() {
        let mut mask = ComponentMask::new();

        let boundary_checks = [
            ComponentId(0),   // Block 0 Lower Bound
            ComponentId(63),  // Block 0 Upper Bound
            ComponentId(64),  // Block 1 Lower Bound
            ComponentId(127), // Block 1 Upper Bound
            ComponentId(128), // Block 2 Lower Bound
            ComponentId(191), // Block 2 Upper Bound
            ComponentId(192), // Block 3 Lower Bound
            ComponentId(255), // Block 3 Upper Bound
        ];

        for &id in &boundary_checks {
            assert!(
                !mask.contains(id),
                "Detected false positive before registration"
            );
            mask.insert(id);
            assert!(
                mask.contains(id),
                "Bit allocation failed to flip register index flag"
            );
            mask.remove(id);
            assert!(
                !mask.contains(id),
                "Flag removal extraction failed to wipe target index"
            );
        }
    }

    #[test]
    fn test_subset_conjunction_matching() {
        let mut entity_archetype = ComponentMask::new();
        entity_archetype.insert(ComponentId(5));
        entity_archetype.insert(ComponentId(85));
        entity_archetype.insert(ComponentId(210));

        let mut passing_query = ComponentMask::new();
        passing_query.insert(ComponentId(5));
        passing_query.insert(ComponentId(210));

        let mut failing_query = ComponentMask::new();
        failing_query.insert(ComponentId(5));
        failing_query.insert(ComponentId(55)); // Missing variant

        assert!(entity_archetype.contains_all(&passing_query));
        assert!(!entity_archetype.contains_all(&failing_query));
    }

    #[test]
    fn test_subset_disjunction_matching() {
        let mut entity_archetype = ComponentMask::new();
        entity_archetype.insert(ComponentId(42));

        let mut passing_query = ComponentMask::new();
        passing_query.insert(ComponentId(99));
        passing_query.insert(ComponentId(42)); // Intersecting target

        let mut failing_query = ComponentMask::new();
        failing_query.insert(ComponentId(1));
        failing_query.insert(ComponentId(254));

        assert!(entity_archetype.contains_any(&passing_query));
        assert!(!entity_archetype.contains_any(&failing_query));
    }

    #[test]
    #[should_panic(expected = "boundary fault")]
    fn test_out_of_bounds_panic_guardrail() {
        let mut mask = ComponentMask::new();
        mask.insert(ComponentId(256));
    }

    #[test]
    fn test_bitwise_coordinate_accuracy() {
        // Verify every single valid ID produces correct coordinates
        for id in 0..MAX_COMPONENTS {
            let expected_block = id / 64;
            let expected_bit = id % 64;

            let (actual_block, actual_bit) = ComponentMask::get_coords(ComponentId(id));

            assert_eq!(actual_block, expected_block, "Block mismatch at ID {}", id);
            assert_eq!(actual_bit, expected_bit, "Bit mismatch at ID {}", id);
        }
    }
}
