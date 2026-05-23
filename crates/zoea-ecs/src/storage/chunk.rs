use crate::error::EcsError;
use crate::storage::column::Column;
use crate::storage::pending_component::PendingComponent;
use crate::topology::component_layout::ComponentLayout;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr::{copy_nonoverlapping, NonNull};
use zoea_core::ecs::entity::EntityId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChunkId(pub usize);

/// Optimal storage limit targeting L1/L2 data cache efficiency limits for heavy simulations.
const CHUNK_MEMORY_SIZE: usize = 16 * 1024;

/// A contiguous 16 KB Structure of Arrays (SoA) data partition storing raw component columns
/// aligned directly to system architectural boundaries.
pub struct Chunk {
    storage: NonNull<u8>,
    layout: Layout,
    columns: Vec<Column>,
    capacity: usize,
    len: usize,
    offsets: Vec<usize>,

    #[cfg(test)]
    pub was_cloned_via_new_from: bool,
}

impl Chunk {
    /// Computes the layout parameters for a chunk based on component descriptions.
    /// Returns the capacity, final memory layout, and calculated byte offsets for each column.
    fn compute_chunk_layout(
        component_layouts: &[ComponentLayout],
    ) -> Result<(usize, Layout, Vec<usize>), EcsError> {
        let mut capacity = 0;
        let mut final_layout = Layout::new::<()>();
        let mut final_offsets = Vec::with_capacity(component_layouts.len());

        // Step 1: Find maximum capacity that fits within CHUNK_MEMORY_SIZE bounds
        loop {
            let next_capacity = capacity + 1;
            let mut current_layout = Layout::array::<EntityId>(next_capacity)
                .map_err(|_| EcsError::LayoutCalculationFailed)?;

            let mut valid = true;
            let mut temporary_offsets = Vec::with_capacity(component_layouts.len());

            for component_layout in component_layouts.iter() {
                let component_array_layout = match Layout::from_size_align(
                    component_layout.size() * next_capacity,
                    component_layout.align(),
                ) {
                    Ok(l) => l,
                    Err(_) => {
                        valid = false;
                        break;
                    }
                };

                if let Ok((new_layout, offset)) = current_layout.extend(component_array_layout) {
                    current_layout = new_layout;
                    temporary_offsets.push(offset);
                } else {
                    valid = false;
                    break;
                }
            }

            let padded_layout = current_layout.pad_to_align();
            if valid && padded_layout.size() <= CHUNK_MEMORY_SIZE {
                capacity = next_capacity;
                final_layout = padded_layout;
                final_offsets = temporary_offsets;
            } else {
                break; // Boundary capacity threshold reached safely
            }
        }

        // Step 2: Handle edge case where a single entry configuration overshoots standard 16 KB bounds
        if capacity == 0 {
            capacity = 1;
            let mut current_layout = Layout::array::<EntityId>(capacity)
                .map_err(|_| EcsError::LayoutCalculationFailed)?;
            final_offsets.clear();

            for component_layout in component_layouts.iter() {
                let component_array_layout = Layout::from_size_align(
                    component_layout.size() * capacity,
                    component_layout.align(),
                )
                .map_err(|_| EcsError::LayoutCalculationFailed)?;

                let (new_layout, offset) = current_layout
                    .extend(component_array_layout)
                    .map_err(|_| EcsError::LayoutCalculationFailed)?;
                current_layout = new_layout;
                final_offsets.push(offset);
            }
            final_layout = current_layout.pad_to_align();
        }

        Ok((capacity, final_layout, final_offsets))
    }

    fn new_alloc(
        capacity: usize,
        layout: Layout,
        offsets: Vec<usize>,
        columns: Vec<Column>,
        #[cfg(test)] was_cloned_via_new_from: bool,
    ) -> Result<Self, EcsError> {
        let ptr = unsafe { alloc(layout) };
        let storage = NonNull::new(ptr).ok_or(EcsError::LayoutCalculationFailed)?;

        Ok(Self {
            storage,
            columns,
            capacity,
            len: 0,
            layout,
            offsets,
            #[cfg(test)]
            was_cloned_via_new_from,
        })
    }

    /// Creates an empty, cleanly allocated twin chunk sharing an identical layout configuration.
    pub fn new_from(origin: &Self) -> Result<Self, EcsError> {
        Self::new_alloc(
            origin.capacity,
            origin.layout,
            origin.offsets.clone(),
            origin.columns.clone(),
            #[cfg(test)]
            true,
        )
    }

    /// Creates a new layout-aligned chunk by dynamically assessing optimal dense
    /// layout storage capacity boundaries for a set of component layout descriptors.
    pub fn new(component_layouts: &[ComponentLayout]) -> Result<Self, EcsError> {
        let (capacity, final_layout, offsets) = Chunk::compute_chunk_layout(component_layouts)?;

        let columns = component_layouts
            .iter()
            .zip(offsets.iter())
            .map(|(component, &offset)| {
                Column::new(
                    offset,
                    component.size(),
                    component.align(),
                    component.drop_fn,
                )
            })
            .collect();

        Self::new_alloc(
            capacity,
            final_layout,
            offsets,
            columns,
            #[cfg(test)]
            false,
        )
    }

    /// Fetches a pointer to a component instance located inside a specific column matrix slot.
    #[inline]
    pub fn get_component_ptr(
        &self,
        component_index: usize,
        index: usize,
    ) -> Result<NonNull<u8>, EcsError> {
        if component_index >= self.columns.len() {
            return Err(EcsError::ColumnIndexOutOfBounds);
        }
        if index >= self.len {
            return Err(EcsError::EntityIndexOutOfBounds);
        }

        let col = self.columns[component_index];

        if col.size() == 0 {
            Ok(NonNull::dangling())
        } else {
            unsafe { Ok(col.get_ptr(self.storage, index)) }
        }
    }

    /// Retrieves the explicit `EntityId` managing the component arrays at the given chunk index.
    #[inline]
    pub fn get_entity_id(&self, index: usize) -> Result<EntityId, EcsError> {
        if index >= self.len {
            return Err(EcsError::EntityIndexOutOfBounds);
        }
        let entity_base_ptr = self.storage.as_ptr() as *const EntityId;
        unsafe { Ok(*entity_base_ptr.add(index)) }
    }

    /// Appends an entity and its raw component tracking wrapper configurations to dense SoA layouts.
    ///
    /// # Safety
    /// Elements mapped inside `components` array descriptors must correspond perfectly to initial structural constraints.
    pub unsafe fn push(
        &mut self,
        entity_id: EntityId,
        components: &[PendingComponent],
    ) -> Result<usize, EcsError> {
        if self.is_full() {
            return Err(EcsError::ChunkIsFull);
        }

        let index = self.len;
        self.len += 1;

        let entity_base_ptr = self.storage.as_ptr() as *mut EntityId;
        unsafe {
            entity_base_ptr.add(index).write(entity_id);
        }

        for (idx, column) in self.columns.iter().enumerate() {
            if column.size() > 0 {
                let data_ptr = components[idx].ptr;
                unsafe {
                    let column_ptr = column.get_ptr(self.storage, index);
                    copy_nonoverlapping(data_ptr.as_ptr(), column_ptr.as_ptr(), column.size());
                }
            }
        }

        Ok(index)
    }

    /// Unwinds an element out of storage layouts via efficient swap-remove, returning
    /// the shifted `EntityId` that inherited the targeted storage offset if one occurred.
    ///
    /// # Safety
    /// Destroys allocations targeting current localized structural index slots.
    pub unsafe fn swap_remove(&mut self, index: usize) -> Result<Option<EntityId>, EcsError> {
        if index >= self.len {
            return Err(EcsError::EntityIndexOutOfBounds);
        }

        let last_index = self.len - 1;

        unsafe {
            for col in self.columns.iter() {
                if col.size() > 0 {
                    let target_ptr = col.get_ptr(self.storage, index);
                    (col.drop_fn)(target_ptr);
                }
            }
        }

        if index == last_index {
            self.len -= 1;
            return Ok(None);
        }

        let entity_base_ptr = self.storage.as_ptr() as *mut EntityId;
        let moved_entity = unsafe { *entity_base_ptr.add(last_index) };

        unsafe {
            copy_nonoverlapping(
                entity_base_ptr.add(last_index),
                entity_base_ptr.add(index),
                1,
            );

            for column in self.columns.iter() {
                if column.size() > 0 {
                    let src = column.get_ptr(self.storage, last_index);
                    let dst = column.get_ptr(self.storage, index);
                    copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), column.size());
                }
            }
        }

        self.len -= 1;
        Ok(Some(moved_entity))
    }

    /// Instantly clears lengths and drops all operational properties stored across raw slots.
    pub fn clear(&mut self) {
        for i in 0..self.len {
            for col in self.columns.iter() {
                unsafe {
                    let ptr = col.get_ptr(self.storage, i);
                    (col.drop_fn)(ptr);
                }
            }
        }
        self.len = 0;
    }

    /// Confirms if operational layout data has maxed tracking capacities.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len == self.capacity
    }

    /// Returns the active number of tracked entities in storage blocks.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the max total number of layout-aligned elements this chunk handles.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Drop for Chunk {
    fn drop(&mut self) {
        self.clear();
        unsafe {
            dealloc(self.storage.as_ptr(), self.layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use std::mem::forget;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_chunk_layout_calculation_and_alignment() {
        let layouts = vec![
            create_layout::<Position>(drop_noop_fn),
            create_layout::<Velocity>(drop_noop_fn),
        ];

        let chunk_res = Chunk::new(&layouts);
        assert!(chunk_res.is_ok(), "Chunk creation failed entirely");

        let chunk = chunk_res.unwrap();
        assert!(chunk.capacity() > 0, "Capacity should be greater than zero");
        assert!(
            chunk.layout.size() <= CHUNK_MEMORY_SIZE,
            "Chunk exceeded maximum memory size boundary"
        );

        // Ensure that each allocated column is properly aligned to the component's requirements
        let _base_address = chunk.storage.as_ptr() as usize;
        for (idx, col) in chunk.columns.iter().enumerate() {
            let column_ptr = unsafe { col.get_ptr(chunk.storage, 0) };
            let column_address = column_ptr.as_ptr() as usize;

            assert_eq!(
                column_address % col.align(),
                0,
                "Column [{}] offset is misaligned to native byte boundary requirements!",
                idx
            );
        }
    }

    #[test]
    fn test_push_and_retrieval_data_flow() {
        let layouts = vec![
            create_layout::<Position>(drop_noop_fn),
            create_layout::<Velocity>(drop_noop_fn),
        ];

        let mut chunk = Chunk::new(&layouts).unwrap();

        let pos = Position { x: 10, y: 20 };
        let vel = Velocity { dx: 1.5, dy: -3.0 };

        // Wrap references into raw PendingComponent instances
        let pending = vec![create_component(pos), create_component(vel)];

        let entity_id = EntityId::new(42, 1);
        let index_res = unsafe { chunk.push(entity_id, &pending) };
        assert!(index_res.is_ok());
        assert_eq!(chunk.len(), 1);

        // Verify Entity ID tracking
        assert_eq!(chunk.get_entity_id(0).unwrap(), EntityId::new(42, 1));

        // Read and cast back type-erased column values
        let raw_pos_ptr = chunk.get_component_ptr(0, 0).unwrap().as_ptr() as *const Position;
        let raw_vel_ptr = chunk.get_component_ptr(1, 0).unwrap().as_ptr() as *const Velocity;

        unsafe {
            assert_eq!(&*raw_pos_ptr, &pos);
            assert_eq!(&*raw_vel_ptr, &vel);
        }
    }

    #[test]
    fn test_swap_remove_mechanics_and_drop_execution() {
        let drop_counter = Arc::new(Mutex::new(0));

        let layouts = vec![create_layout::<DroppableComponent>(
            drop_component_fn::<DroppableComponent>,
        )];

        let mut chunk = Chunk::new(&layouts).unwrap();

        // Push 3 structural elements into tracking
        for i in 0..3 {
            let comp = DroppableComponent {
                counter: drop_counter.clone(),
            };
            let pending = vec![create_component(comp)];
            unsafe { chunk.push(EntityId::new(i, 1), &pending).unwrap() };
            // Forget local stack tracking to let the chunk handle exclusive ownership
            forget(drop_counter.clone());
        }

        assert_eq!(chunk.len(), 3);
        assert_eq!(
            *drop_counter.lock().unwrap(),
            3, // Drops occurs during the pending vector destruction
            "No components should be dropped yet"
        );

        // Swap remove middle element (Index 1, EntityId(1))
        // This must drop Index 1, and shift Index 2 (EntityId(2)) down into Index 1.
        let remove_res = unsafe { chunk.swap_remove(1) };
        assert!(remove_res.is_ok());

        // Ensure the returned structural optimization points to EntityId(2) moving up
        assert_eq!(remove_res.unwrap(), Some(EntityId::new(2, 1)));
        assert_eq!(chunk.len(), 2);
        assert_eq!(
            *drop_counter.lock().unwrap(),
            4, // 3 Drops occurs during the pending vector destruction + 1 for swap_remove
            "Exactly 1 component layout should have executed drop handles"
        );

        // Verify remaining structure alignment matches changes
        assert_eq!(chunk.get_entity_id(0).unwrap(), EntityId::new(0, 1));
        assert_eq!(
            chunk.get_entity_id(1).unwrap(),
            EntityId::new(2, 1),
            "Entity 2 should have shifted to index 1"
        );
    }

    #[test]
    fn test_chunk_clear_and_raii_drop_unwinding() {
        let drop_counter = Arc::new(Mutex::new(0));

        let layouts = vec![create_layout::<DroppableComponent>(
            drop_component_fn::<DroppableComponent>,
        )];

        {
            let mut chunk = Chunk::new(&layouts).unwrap();
            for i in 0..5 {
                let comp = DroppableComponent {
                    counter: drop_counter.clone(),
                };
                let pending = vec![create_component(comp)];
                unsafe { chunk.push(EntityId::new(i, 1), &pending).unwrap() };
                forget(drop_counter.clone());
            }
            assert_eq!(chunk.len(), 5);
            // Chunk gets dropped here out of scope boundaries
        }

        assert_eq!(
            *drop_counter.lock().unwrap(),
            10, // 5 drop during création + 5 drops for destruction
            "Chunk drop must thoroughly clean all active structural column elements!"
        );
    }

    #[test]
    fn test_out_of_bounds_error_handling() {
        let layouts = vec![create_layout::<Position>(drop_noop_fn)];

        let chunk = Chunk::new(&layouts).unwrap();

        // Ask for columns or elements that don't exist yet
        assert_eq!(
            chunk.get_component_ptr(99, 0).unwrap_err(),
            EcsError::ColumnIndexOutOfBounds
        );
        assert_eq!(
            chunk.get_component_ptr(0, 0).unwrap_err(),
            EcsError::EntityIndexOutOfBounds
        );
        assert_eq!(
            chunk.get_entity_id(0).unwrap_err(),
            EcsError::EntityIndexOutOfBounds
        );
    }
}
