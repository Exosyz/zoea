use std::ptr::NonNull;

/// A layout-erased column descriptor that calculates memory addresses
/// for specific component arrays inside a contiguous memory chunk.
#[derive(Copy, Clone)]
pub struct Column {
    offset: usize,
    element_size: usize,
    element_align: usize,
    pub drop_fn: unsafe fn(NonNull<u8>),
}

impl Column {
    /// Creates a new layout-erased column descriptor.
    ///
    /// *Performance: $O(1)$ — Inline compile-time constant creation.*
    #[inline]
    pub fn new(
        offset: usize,
        element_size: usize,
        element_align: usize,
        drop_fn: unsafe fn(NonNull<u8>),
    ) -> Self {
        Self {
            offset,
            element_size,
            element_align,
            drop_fn,
        }
    }

    /// Computes the read-only raw pointer to the element at the specified index
    /// within a structural data chunk.
    ///
    /// # Safety
    /// * `chunk_ptr` must point to a valid allocation containing this column's data.
    /// * The calculated address (`offset + index * size`) must fall within the bounds
    ///   of the allocated memory block.
    /// * The `chunk_ptr` must be properly aligned according to this column's internal requirements.
    #[inline]
    pub unsafe fn get_ptr(&self, chunk_ptr: NonNull<u8>, index: usize) -> NonNull<u8> {
        // Safety check: pointer addition remains completely in-bounds of the active allocation
        unsafe { chunk_ptr.add(self.offset + (index * self.element_size)) }
    }

    /// Returns the uniform size in bytes of an individual element in this column.
    #[inline]
    pub const fn size(&self) -> usize {
        self.element_size
    }

    /// Returns the alignment in bytes required by an individual element in this column.
    #[inline]
    pub const fn align(&self) -> usize {
        self.element_align
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{alloc, dealloc, Layout};

    #[derive(Default, Debug, PartialEq)]
    struct Velocity {
        dx: f32,
        dy: f32,
    }
    impl zoea_core::ecs::component::Component for Velocity {}

    unsafe fn drop_noop(_: NonNull<u8>) {}

    #[test]
    fn test_non_null_column_calculations() {
        let size = size_of::<Velocity>();
        let align = align_of::<Velocity>();
        let offset = 32; // Simulating a well-aligned offset in an Archetype chunk

        let column = Column::new(offset, size, align, drop_noop);
        let chunk_layout = Layout::from_size_align(128, 16).unwrap();

        unsafe {
            let raw_alloc = alloc(chunk_layout);
            let chunk_ptr = NonNull::new(raw_alloc).expect("Allocation failed");

            // Compute structural offsets manually to write raw test inputs
            let ptr_idx_0 = chunk_ptr.add(offset).as_ptr() as *mut Velocity;
            let ptr_idx_1 = chunk_ptr.add(offset + size).as_ptr() as *mut Velocity;

            std::ptr::write(ptr_idx_0, Velocity { dx: 1.5, dy: -3.0 });
            std::ptr::write(ptr_idx_1, Velocity { dx: 0.0, dy: 10.0 });

            // Validate that get_ptr matches our manual layouts completely using NonNull
            let resolved_0 = column.get_ptr(chunk_ptr, 0);
            let resolved_1 = column.get_ptr(chunk_ptr, 1);

            assert_eq!(
                resolved_0.as_ptr() as *const Velocity,
                ptr_idx_0 as *const Velocity
            );
            assert_eq!(resolved_1.as_ptr() as *mut Velocity, ptr_idx_1);

            // Cast the type-erased pointers (*mut u8) to explicit typed pointers (*const Velocity)
            let typed_ptr_0 = resolved_0.as_ptr() as *const Velocity;
            let typed_ptr_1 = resolved_1.as_ptr() as *const Velocity;

            // Now you can safely use read or dereference them inside the unsafe block
            assert_eq!(&*typed_ptr_0, &Velocity { dx: 1.5, dy: -3.0 });
            assert_eq!(&*typed_ptr_1, &Velocity { dx: 0.0, dy: 10.0 });

            dealloc(raw_alloc, chunk_layout);
        }
    }
}
