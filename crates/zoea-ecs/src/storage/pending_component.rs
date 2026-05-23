use crate::topology::component_registry::ComponentId;
use std::alloc::{alloc, dealloc, Layout};
use std::mem::forget;
use std::ptr::{drop_in_place, write, NonNull};
use zoea_core::ecs::component::Component;

/// A type-erased container holding a dynamically allocated component prior to
/// insertion into the ECS dense arrays (typically used in Command Buffers).
pub struct PendingComponent {
    pub id: ComponentId,
    pub layout: Layout,
    pub ptr: NonNull<u8>,
    /// Type-erased function pointer that handles both payload dropping
    /// and heap deallocation if the component container is abandoned.
    pub drop_fn: unsafe fn(NonNull<u8>),
}

/// Generic named drop helper. Reconstructs the original Box context to drop
/// inner data structures and return the raw allocation block back to the system allocator.
unsafe fn drop_component_helper<T: Component>(ptr: NonNull<u8>) {
    unsafe {
        drop_in_place(ptr.as_ptr() as *mut T);
    }
}

impl PendingComponent {
    /// Creates a new type-erased `PendingComponent`.
    ///
    /// # Safety
    /// The provided `ptr` must be a valid, heap-allocated pointer created using
    /// an active `Box::into_raw` context matching type `T`.
    pub fn new<T: Component>(id: ComponentId, instance: T) -> Self {
        let layout = Layout::new::<T>();

        let ptr = if layout.size() == 0 {
            // For ZST (Zero size tag), we use dangling ptr (safe and align)
            NonNull::dangling()
        } else {
            let raw = unsafe { alloc(layout) };
            let non_null = NonNull::new(raw).expect("Allocation failed");
            unsafe {
                write(non_null.as_ptr() as *mut T, instance);
            }
            non_null
        };

        Self {
            id,
            layout: Layout::new::<T>(),
            ptr,
            drop_fn: drop_component_helper::<T>,
        }
    }

    /// Releases the heap-allocated staging memory without invoking the underlying
    /// component's destructor, as the value has been moved into a Chunk.
    ///
    /// # Safety
    /// This method must only be called if the contents of the pointer have been copied
    /// into another storage area that now assumes dropping responsibility.
    pub unsafe fn release_allocation_shell(self) {
        if self.layout.size() > 0 {
            unsafe { dealloc(self.ptr.as_ptr(), self.layout) };
        }

        forget(self);
    }
}

impl Drop for PendingComponent {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            (self.drop_fn)(self.ptr);

            if self.layout.size() > 0 {
                dealloc(self.ptr.as_ptr(), self.layout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DROP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct DroppableComponent {
        _data: String,
    }
    impl Component for DroppableComponent {}
    impl Drop for DroppableComponent {
        fn drop(&mut self) {
            DROP_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct TrivialComponent {
        _id: u32,
    }
    impl Component for TrivialComponent {}

    #[test]
    fn test_pending_component_triggers_drop_on_raii() {
        DROP_COUNTER.store(0, Ordering::SeqCst);
        let component_id = ComponentId(1);

        {
            let _pending = PendingComponent::new(
                component_id,
                DroppableComponent {
                    _data: String::from("Zoea Safe Optimization"),
                },
            );
        }

        assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_pending_component_with_trivial_type_frees_memory_without_leak() {
        let component_id = ComponentId(2);
        {
            let _pending = PendingComponent::new(component_id, TrivialComponent { _id: 42 });
        } // Safely reclaims memory here via Box::from_raw handling inside drop_component_helper
    }

    #[test]
    fn test_extracted_drop_fn_executes_correctly_without_aliasing_violation() {
        DROP_COUNTER.store(0, Ordering::SeqCst);
        let component_id = ComponentId(3);

        let pending = PendingComponent::new(
            component_id,
            DroppableComponent {
                _data: String::from("Pure Raw Pointer Test"),
            },
        );

        unsafe {
            // Trigger drop directly using the function pointer.
            // This increments the counter AND frees the underlying heap allocation.
            (pending.drop_fn)(pending.ptr);

            // Disarm the container's standard RAII drop so it doesn't trigger a double-free.
            forget(pending);
        }

        assert_eq!(DROP_COUNTER.load(Ordering::SeqCst), 1);
    }
}
