//! Temporary type-erased component containers for staging and transactional ingestion.
//!
//! Provides `PendingComponent`, a structure that wraps dynamically added components
//! on the heap with type-erased pointers and custom destructors, ensuring safe lifecycle
//! tracking prior to array consolidation.

use crate::topology::component_registry::ComponentId;
use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::mem::forget;
use std::ptr::{drop_in_place, write, NonNull};
use zoea_core::ecs::component::Component;

/// A type-erased heap staging container holding an initialized component instance
/// along with its layout characteristics and dynamic destructor.
pub struct PendingComponent {
    pub id: ComponentId,
    pub ptr: NonNull<u8>,
    pub layout: Layout,
    pub drop_fn: unsafe fn(NonNull<u8>),
}

impl PendingComponent {
    /// Boxes a component instance onto the heap, returning a type-erased tracking descriptor.
    pub fn new<T: Component>(id: ComponentId, value: T) -> Self {
        let layout = Layout::new::<T>();

        if layout.size() == 0 {
            return Self {
                id,
                ptr: NonNull::dangling(),
                layout,
                drop_fn: drop_component_helper::<T>,
            };
        }

        let raw = unsafe { alloc(layout) };
        if raw.is_null() {
            handle_alloc_error(layout);
        }

        unsafe {
            write(raw as *mut T, value);
        }

        Self {
            id,
            ptr: NonNull::new(raw).unwrap(),
            layout,
            drop_fn: drop_component_helper::<T>,
        }
    }

    /// Releases the temporary staging allocation box without invoking the component's destructor.
    ///
    /// Crucial when handing off component data payloads to dense contiguous storage arrays,
    /// preventing staging memory leaks while leaving the copied data intact.
    pub unsafe fn release_allocation_shell(self) {
        if self.layout.size() > 0 {
            unsafe { dealloc(self.ptr.as_ptr(), self.layout) };
        }
        forget(self);
    }

    /// Custom deep-cloning routine for testing scenarios to safely replicate staging data
    /// without inducing raw pointer double-frees or aliasing conflicts.
    #[cfg(test)]
    pub fn test_clone<T: Component + Clone>(&self) -> Self {
        unsafe {
            let source_val = &*(self.ptr.as_ptr() as *const T);
            Self::new::<T>(self.id, source_val.clone())
        }
    }
}

impl Drop for PendingComponent {
    fn drop(&mut self) {
        if self.layout.size() > 0 {
            unsafe {
                // Call the type-erased destructor function pointer first
                (self.drop_fn)(self.ptr);
                // Free the backing layout box memory
                dealloc(self.ptr.as_ptr(), self.layout);
            }
        }
    }
}

/// Type-erased helper hook used to cleanly invoke standard `Drop::drop` implementations.
pub unsafe fn drop_component_helper<T: Component>(ptr: NonNull<u8>) {
    unsafe { drop_in_place(ptr.as_ptr() as *mut T) };
}

#[cfg(test)]
mod tests {
    use super::*;
    // Integrated directly with your shared test utility file
    use crate::test_utils::{DroppableComponent, Position};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_pending_component_triggers_drop_on_raii() {
        let counter = Arc::new(Mutex::new(0));
        {
            let comp = DroppableComponent {
                counter: counter.clone(),
            };
            let _pending = PendingComponent::new(ComponentId(1), comp);
        }
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[test]
    fn test_pending_component_with_trivial_type_frees_memory_without_leak() {
        {
            let _pending = PendingComponent::new(ComponentId(2), Position::from(10));
        }
        // Miri confirms that memory allocations for primitive/copy types are successfully reclaimed
    }

    #[test]
    fn test_extracted_drop_fn_executes_correctly_without_aliasing_violation() {
        let counter = Arc::new(Mutex::new(0));
        let comp = DroppableComponent {
            counter: counter.clone(),
        };
        let pending = PendingComponent::new(ComponentId(3), comp);

        unsafe {
            // Execute the dynamic type-erased destructor function pointer directly
            (pending.drop_fn)(pending.ptr);
        }

        assert_eq!(*counter.lock().unwrap(), 1);

        // Explicitly clear the underlying raw container allocation shell
        // to prevent staging memory leaks during manually unrolled tests
        unsafe {
            if pending.layout.size() > 0 {
                dealloc(pending.ptr.as_ptr(), pending.layout);
            }
        }
        forget(pending);
    }
}
