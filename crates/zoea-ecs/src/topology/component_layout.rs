//! Lightweight layout metadata descriptors tracking structural footprints and destructors.
//!
//! A `ComponentLayout` stores size, hardware alignment, and type-erased destructor function
//! pointers for a specific component type. It is stripped of raw pointers, making it safe to copy,
//! share, and retain inside dense storage layouts like Archetypes or Tables.

use crate::storage::pending_component::{drop_component_helper, PendingComponent};
use crate::topology::component_registry::ComponentId;
use std::alloc::Layout;
use std::ptr::NonNull;
use zoea_core::ecs::component::Component;

/// A lightweight, copyable metadata descriptor defining the structural
/// footprint and destructor behavior of a specific component type.
#[derive(Copy, Clone, Debug)]
pub struct ComponentLayout {
    pub id: ComponentId,
    pub layout: Layout,
    pub align: usize,
    pub drop_fn: unsafe fn(NonNull<u8>),
}

impl PartialEq for ComponentLayout {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ComponentLayout {}

impl From<&PendingComponent> for ComponentLayout {
    #[inline]
    fn from(pending: &PendingComponent) -> Self {
        Self {
            id: pending.id,
            layout: pending.layout,
            drop_fn: pending.drop_fn,
            align: pending.align,
        }
    }
}

impl ComponentLayout {
    /// Instantiates a new component structural layout metadata block for type `T`.
    pub fn new<T: Component>(id: ComponentId) -> Self {
        Self {
            id,
            layout: Layout::new::<T>(),
            drop_fn: drop_component_helper::<T>,
            align: T::ALIGNMENT,
        }
    }

    /// Returns the exact size in bytes required by this component type.
    ///
    /// *Performance: O(1) — Inline compile-time constant lookup.*
    #[inline]
    pub const fn size(&self) -> usize {
        self.layout.size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::dealloc;
    use std::mem::forget;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Global atomic counter tracking component destructor calls
    static COMPONENT_DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    // A heavy test component type with a strict explicit alignment requirement
    #[repr(align(16))]
    #[derive(Clone)]
    struct ManagedVector {
        _x: f32,
        _y: f32,
        _z: f32,
        _w: f32,
    }

    impl Component for ManagedVector {}

    impl Drop for ManagedVector {
        fn drop(&mut self) {
            COMPONENT_DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_layout_extraction_matches_raw_type_properties() {
        let expected_layout = Layout::new::<ManagedVector>();

        let instance = ManagedVector {
            _x: 0.0,
            _y: 0.0,
            _z: 0.0,
            _w: 0.0,
        };

        let pending = PendingComponent::new::<ManagedVector>(ComponentId(42), instance);
        let component_layout = ComponentLayout::from(&pending);

        assert_eq!(component_layout.id, ComponentId(42));
        assert_eq!(
            component_layout.size(),
            expected_layout.size(),
            "Extracted size metadata drifted from type configuration"
        );
        assert_eq!(
            component_layout.align, 16,
            "Hardware alignment requirement was lost during conversion"
        );
    }

    #[test]
    fn test_extracted_drop_fn_executes_correctly() {
        COMPONENT_DROP_COUNT.store(0, Ordering::SeqCst);

        let instance = ManagedVector {
            _x: 1.0,
            _y: 2.0,
            _z: 3.0,
            _w: 4.0,
        };

        let pending = PendingComponent::new::<ManagedVector>(ComponentId(100), instance);
        let component_layout = ComponentLayout::from(&pending);

        unsafe {
            (component_layout.drop_fn)(pending.ptr);
        }

        assert_eq!(COMPONENT_DROP_COUNT.load(Ordering::SeqCst), 1);

        unsafe {
            if pending.layout.size() > 0 {
                dealloc(pending.ptr.as_ptr(), pending.layout);
            }
        }
        forget(pending);
    }
}
