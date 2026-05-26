//! Contiguous architecture for low-level Data-Oriented SoA (Structure of Arrays) partitions.
//!
//! Chunks manage type-erased component columns optimized directly for hardware L1/L2
//! data cache lines, preventing memory fragmentation via continuous swap-remove operations.

use crate::error::EcsError;
use crate::storage::column::Column;
use crate::topology::component_layout::ComponentLayout;
use std::alloc::{alloc, dealloc, Layout};
use std::cmp::max;
use std::mem::size_of;
use std::ptr::{copy_nonoverlapping, NonNull};
use zoea_core::ecs::entity::EntityId;

/// Unique identifier marking a specific localized 16 KB data chunk allocation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChunkId(pub usize);

/// Optimal storage limit targeting L1/L2 data cache efficiency limits for heavy simulations.
#[cfg(not(test))]
const CHUNK_MEMORY_SIZE: usize = 16 * 1024; // 16Kb
#[cfg(test)]
const CHUNK_MEMORY_SIZE: usize = 1024; // 1Kb

/// A contiguous 16 KB Structure of Arrays (SoA) data partition storing raw component columns
/// aligned directly to system architectural boundaries.
pub struct Chunk {
    /// Raw un-typed master pointer pointing to the base layout allocation shell.
    storage: NonNull<u8>,
    /// Global memory blueprint layout mapping the total size and alignment criteria of this allocation.
    layout: Layout,
    /// Ordered collection of column metadata wrappers detailing inner offset jumps.
    columns: Vec<Column>,
    /// High-water density threshold representing the max entity capacity configuration.
    capacity: usize,
    /// Active tracking register for the current number of initialized entity slots filled.
    len: usize,
    /// Direct byte jumps from the storage base address indicating where columns begin.
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
        // 1. Calculate an optimistic upper bound (ignoring alignment padding)
        let base_entity_size = size_of::<EntityId>();
        let components_size: usize = component_layouts.iter().map(|c| c.size()).sum();

        // Prevent divide-by-zero just in case (though EntityId ensures it's > 0)
        let total_size = max(1, base_entity_size + components_size);

        let mut capacity = CHUNK_MEMORY_SIZE / total_size;
        if capacity == 0 {
            capacity = 1; // Fallback for massive components > 16 KB
        }

        // 2. Step downwards from the upper bound to account for padding
        loop {
            let mut current_layout = Layout::array::<EntityId>(capacity)
                .map_err(|_| EcsError::LayoutCalculationFailed)?;
            let mut offsets = Vec::with_capacity(component_layouts.len());
            let mut valid = true;

            for comp in component_layouts {
                let alignment = comp.align;
                let array_layout = Layout::from_size_align(comp.size() * capacity, alignment)
                    .map_err(|_| EcsError::LayoutCalculationFailed)?;

                if let Ok((new_layout, offset)) = current_layout.extend(array_layout) {
                    current_layout = new_layout;
                    offsets.push(offset);
                } else {
                    valid = false;
                    break;
                }
            }

            if valid {
                let final_layout = current_layout.pad_to_align();

                // If it fits within 16KB, OR if capacity is already reduced to 1 (the minimum viable size), we accept it.
                if final_layout.size() <= CHUNK_MEMORY_SIZE || capacity == 1 {
                    return Ok((capacity, final_layout, offsets));
                }
            }

            // If we overshoot due to padding, decrement capacity and try again.
            // This will typically only run 1-2 times, rather than thousands of times.
            capacity -= 1;
        }
    }

    /// Internal helper to request system memory and construct the allocation context shell.
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
                Column::new(offset, component.size(), component.align, component.drop_fn)
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
            unsafe { Ok(col.get_component_ptr(self.storage, index)) }
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

    pub unsafe fn get_column_slice_info(&self, component_index: usize) -> (NonNull<u8>, usize) {
        let ptr = unsafe { self.columns[component_index].get_ptr(self.storage) };

        (ptr, self.len)
    }

    /// Appends an entity and its raw component pointer configurations to dense SoA layouts.
    ///
    /// # Safety
    /// * `components` raw pointers must perfectly mirror this chunk's sorted column sizes and alignments.
    /// * The caller must pass ownership of the pointed data blocks to this chunk and avoid dropping them on the stack.
    pub unsafe fn push(
        &mut self,
        entity_id: EntityId,
        components: &[NonNull<u8>],
    ) -> Result<usize, EcsError> {
        if self.is_full() {
            return Err(EcsError::ChunkIsFull);
        }
        if components.len() < self.columns.len() {
            return Err(EcsError::ColumnIndexOutOfBounds);
        }

        let index = self.len;

        let entity_base_ptr = self.storage.as_ptr() as *mut EntityId;
        unsafe { entity_base_ptr.add(index).write(entity_id) };

        for (idx, column) in self.columns.iter().enumerate() {
            if column.size() > 0 {
                let data_ptr = components[idx];
                unsafe {
                    let column_ptr = column.get_component_ptr(self.storage, index);
                    copy_nonoverlapping(data_ptr.as_ptr(), column_ptr.as_ptr(), column.size());
                }
            }
        }

        self.len += 1;
        Ok(index)
    }

    /// Unwinds an element out of storage layouts via efficient swap-remove, returning
    /// the shifted `EntityId` that inherited the targeted storage offset if one occurred.
    ///
    /// # Safety
    /// Destroys allocations targeting current localized structural index slots without running drops.
    /// Component values located at the target index are left unmanaged and must be manually handled by the caller.
    pub unsafe fn swap_remove_and_forget(
        &mut self,
        index: usize,
    ) -> Result<Option<EntityId>, EcsError> {
        if index >= self.len {
            return Err(EcsError::EntityIndexOutOfBounds);
        }

        let last_index = self.len - 1;

        if index == last_index {
            self.len -= 1;
            return Ok(None);
        }

        let entity_base_ptr = self.storage.as_ptr() as *mut EntityId;
        let moved_entity = *entity_base_ptr.add(last_index);

        unsafe { *entity_base_ptr.add(index) = moved_entity };

        for column in self.columns.iter() {
            if column.size() > 0 {
                unsafe {
                    let src = column.get_component_ptr(self.storage, last_index);
                    let dst = column.get_component_ptr(self.storage, index);
                    copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), column.size());
                }
            }
        }

        self.len -= 1;
        Ok(Some(moved_entity))
    }

    /// Permanently destroys an entity and safely drops all of its components.
    ///
    /// # Safety
    /// Invalidates pointers referencing values positioned at the targeted structural index slot.
    pub unsafe fn swap_remove_and_drop(
        &mut self,
        index: usize,
    ) -> Result<Option<EntityId>, EcsError> {
        if index >= self.len {
            return Err(EcsError::EntityIndexOutOfBounds);
        }

        self.drop_at(index);

        unsafe { self.swap_remove_and_forget(index) }
    }

    fn drop_at(&mut self, index: usize) {
        for col in self.columns.iter() {
            if col.size() > 0 {
                unsafe {
                    let ptr = col.get_component_ptr(self.storage, index);
                    (col.drop_fn)(ptr);
                }
            }
        }
    }

    /// Instantly clears lengths and drops all operational properties stored across raw slots.
    pub fn clear(&mut self) {
        let active_len = self.len;
        self.len = 0; // State is rolled back immediately to prevent double drop artifacts on failure.

        for i in 0..active_len {
            self.drop_at(i)
        }
    }

    /// Extracts structural row data slices into sequential pointer forms.
    ///
    /// # Safety
    /// Lifetimes of returned pointers are directly bound to the integrity of this chunk's physical buffer layout.
    pub unsafe fn extract_entity(&mut self, index: usize) -> Result<Vec<NonNull<u8>>, EcsError> {
        if index >= self.len {
            return Err(EcsError::EntityIndexOutOfBounds);
        }

        let mut ptrs = Vec::with_capacity(self.columns.len() + 1);
        let id_ptr_raw = unsafe { (self.storage.as_ptr() as *mut EntityId).add(index) };
        let id_ptr = NonNull::new(id_ptr_raw as *mut u8).ok_or(EcsError::InternalError)?;
        ptrs.push(id_ptr);

        for column in self.columns.iter() {
            let ptr = if column.size() == 0 {
                NonNull::dangling()
            } else {
                unsafe { column.get_component_ptr(self.storage, index) }
            };
            ptrs.push(ptr);
        }

        Ok(ptrs)
    }

    /// Injections raw pointer data sequences into chunk arrays.
    ///
    /// # Safety
    /// * `ptrs` must contain exactly `self.columns.len() + 1` pointers.
    /// * `ptrs[0]` must map to a valid `EntityId`.
    /// * `ptrs[1..]` must link to continuous fields mirroring column alignments perfectly.
    pub unsafe fn inject_entity(&mut self, ptrs: Vec<NonNull<u8>>) -> Result<usize, EcsError> {
        if self.is_full() {
            return Err(EcsError::ChunkIsFull);
        }
        if ptrs.len() != self.columns.len() + 1 {
            return Err(EcsError::InternalError);
        }

        let index = self.len;

        let entity_base_ptr = self.storage.as_ptr() as *mut EntityId;
        let entity_ptr = unsafe { entity_base_ptr.add(index) };
        let data_ptr = ptrs[0];

        unsafe {
            copy_nonoverlapping(
                data_ptr.as_ptr(),
                entity_ptr as *mut u8,
                size_of::<EntityId>(),
            );
        }

        for (idx, column) in self.columns.iter().enumerate() {
            if column.size() > 0 {
                let data_ptr = ptrs[idx + 1];
                unsafe {
                    let column_ptr = column.get_component_ptr(self.storage, index);
                    copy_nonoverlapping(data_ptr.as_ptr(), column_ptr.as_ptr(), column.size());
                }
            }
        }

        self.len += 1;
        Ok(index)
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

        let chunk = Chunk::new(&layouts).expect("Chunk creation failed entirely");
        assert!(chunk.capacity() > 0, "Capacity should be greater than zero");
        assert!(
            chunk.layout.size() <= CHUNK_MEMORY_SIZE,
            "Chunk exceeded maximum memory size boundary"
        );

        for (idx, col) in chunk.columns.iter().enumerate() {
            let column_ptr = unsafe { col.get_component_ptr(chunk.storage, 0) };
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

        let raw_ptrs = vec![
            NonNull::new(&pos as *const Position as *mut u8).unwrap(),
            NonNull::new(&vel as *const Velocity as *mut u8).unwrap(),
        ];

        let entity_id = EntityId::new(42, 1);
        let index_res = unsafe { chunk.push(entity_id, &raw_ptrs) };
        assert!(index_res.is_ok());
        assert_eq!(chunk.len(), 1);

        assert_eq!(chunk.get_entity_id(0).unwrap(), EntityId::new(42, 1));

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

        let mut components = Vec::new();
        for _ in 0..3 {
            components.push(DroppableComponent {
                counter: drop_counter.clone(),
            });
        }

        for (i, comp) in components.iter().enumerate() {
            let raw_ptrs =
                vec![NonNull::new(comp as *const DroppableComponent as *mut u8).unwrap()];
            unsafe { chunk.push(EntityId::new(i as u32, 1), &raw_ptrs).unwrap() };
        }

        *drop_counter.lock().unwrap() = 0;
        assert_eq!(chunk.len(), 3);

        let remove_res = unsafe { chunk.swap_remove_and_forget(1) };
        assert!(remove_res.is_ok());

        assert_eq!(remove_res.unwrap(), Some(EntityId::new(2, 1)));
        assert_eq!(chunk.len(), 2);
        assert_eq!(
            *drop_counter.lock().unwrap(),
            0,
            "swap_remove should move memory without dropping"
        );

        unsafe {
            let ptr = chunk.get_component_ptr(0, 1).unwrap();
            (layouts[0].drop_fn)(ptr);
        }

        assert_eq!(
            *drop_counter.lock().unwrap(),
            1,
            "Drop should work manually"
        );

        for comp in components {
            forget(comp);
        }
    }

    #[test]
    fn test_chunk_clear_and_raii_drop_unwinding() {
        let drop_counter = Arc::new(Mutex::new(0));

        let layouts = vec![create_layout::<DroppableComponent>(
            drop_component_fn::<DroppableComponent>,
        )];

        let mut components = Vec::new();
        for _ in 0..5 {
            components.push(DroppableComponent {
                counter: drop_counter.clone(),
            });
        }

        {
            let mut chunk = Chunk::new(&layouts).unwrap();
            for (i, comp) in components.iter().enumerate() {
                let raw_ptrs =
                    vec![NonNull::new(comp as *const DroppableComponent as *mut u8).unwrap()];
                unsafe { chunk.push(EntityId::new(i as u32, 1), &raw_ptrs).unwrap() };
            }
            assert_eq!(chunk.len(), 5);
        }

        assert_eq!(
            *drop_counter.lock().unwrap(),
            5,
            "Chunk drop must thoroughly clean all active structural column elements!"
        );

        for comp in components {
            forget(comp);
        }
    }

    #[test]
    fn test_out_of_bounds_error_handling() {
        let layouts = vec![create_layout::<Position>(drop_noop_fn)];
        let chunk = Chunk::new(&layouts).unwrap();

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

    #[test]
    fn test_chunk_inject_and_extract_single_component() {
        let layout_pos = create_layout::<Position>(drop_noop_fn);
        let mut chunk = Chunk::new(&[layout_pos]).unwrap();

        let entity_id = EntityId::new(100, 1);
        let pos_component = Position { x: 5, y: 10 };

        let ptrs_to_inject = vec![
            NonNull::new(&entity_id as *const EntityId as *mut u8).unwrap(),
            NonNull::new(&pos_component as *const Position as *mut u8).unwrap(),
        ];

        let index = unsafe {
            chunk
                .inject_entity(ptrs_to_inject)
                .expect("Injection failed")
        };

        assert_eq!(index, 0);
        assert_eq!(chunk.len(), 1);

        let extracted_ptrs = unsafe { chunk.extract_entity(index).expect("Extraction failed") };
        assert_eq!(extracted_ptrs.len(), 2);

        unsafe {
            let extracted_id = *(extracted_ptrs[0].as_ptr() as *const EntityId);
            assert_eq!(extracted_id, entity_id);

            let extracted_pos = *(extracted_ptrs[1].as_ptr() as *const Position);
            assert_eq!(extracted_pos, Position { x: 5, y: 10 });
        }
    }

    #[test]
    fn test_chunk_inject_and_extract_multiple_components() {
        let layout_pos = create_layout::<Position>(drop_noop_fn);
        let layout_vel = create_layout::<Velocity>(drop_noop_fn);
        let mut chunk = Chunk::new(&[layout_pos, layout_vel]).unwrap();

        let entity_id = EntityId::new(42, 1);
        let pos_comp = Position { x: 1, y: 2 };
        let vel_comp = Velocity { dx: 0.5, dy: 1.5 };

        let ptrs_to_inject = vec![
            NonNull::new(&entity_id as *const EntityId as *mut u8).unwrap(),
            NonNull::new(&pos_comp as *const Position as *mut u8).unwrap(),
            NonNull::new(&vel_comp as *const Velocity as *mut u8).unwrap(),
        ];

        let index = unsafe { chunk.inject_entity(ptrs_to_inject).unwrap() };
        let extracted_ptrs = unsafe { chunk.extract_entity(index).unwrap() };

        assert_eq!(extracted_ptrs.len(), 3);

        unsafe {
            let extracted_id = *(extracted_ptrs[0].as_ptr() as *const EntityId);
            let extracted_pos = *(extracted_ptrs[1].as_ptr() as *const Position);
            let extracted_vel = *(extracted_ptrs[2].as_ptr() as *const Velocity);

            assert_eq!(extracted_id, entity_id);
            assert_eq!(extracted_pos, Position { x: 1, y: 2 });
            assert_eq!(extracted_vel, Velocity { dx: 0.5, dy: 1.5 });
        }
    }

    #[test]
    fn test_chunk_sequential_injection() {
        let layout_pos = create_layout::<Position>(drop_noop_fn);
        let mut chunk = Chunk::new(&[layout_pos]).unwrap();

        let e1_id = EntityId::new(1, 1);
        let e1_pos = Position { x: 10, y: 10 };
        let idx1 = unsafe {
            chunk
                .inject_entity(vec![
                    NonNull::new(&e1_id as *const EntityId as *mut u8).unwrap(),
                    NonNull::new(&e1_pos as *const Position as *mut u8).unwrap(),
                ])
                .unwrap()
        };

        let e2_id = EntityId::new(2, 1);
        let e2_pos = Position { x: 20, y: 20 };
        let idx2 = unsafe {
            chunk
                .inject_entity(vec![
                    NonNull::new(&e2_id as *const EntityId as *mut u8).unwrap(),
                    NonNull::new(&e2_pos as *const Position as *mut u8).unwrap(),
                ])
                .unwrap()
        };

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(chunk.len(), 2);

        unsafe {
            let ptrs = chunk.extract_entity(idx2).unwrap();
            let pos = *(ptrs[1].as_ptr() as *const Position);
            assert_eq!(pos, Position { x: 20, y: 20 });
        }
    }
}
