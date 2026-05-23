use crate::storage::pending_component::PendingComponent;
use crate::topology::component_registry::ComponentId;
use std::alloc::Layout;
use std::ptr::NonNull;

/// A lightweight, copyable metadata descriptor defining the structural
/// footprint and destructor behavior of a specific component type.
///
/// Stripped of raw pointers, this structure is safe to share, duplicate,
/// and store alongside dense storage backends (e.g., Archetypes or Table views)
/// to dynamically handle raw memory allocations and dynamic drops.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ComponentLayout {
    pub id: ComponentId,
    pub layout: Layout,
    pub drop_fn: unsafe fn(NonNull<u8>),
}

impl From<&PendingComponent> for ComponentLayout {
    #[inline]
    fn from(pending: &PendingComponent) -> Self {
        Self {
            id: pending.id,
            layout: pending.layout,
            drop_fn: pending.drop_fn,
        }
    }
}

impl ComponentLayout {
    /// Returns the exact size in bytes required by this component type.
    ///
    /// *Performance: $O(1)$ — Inline compile-time constant lookup.*
    #[inline]
    pub const fn size(&self) -> usize {
        self.layout.size()
    }

    /// Returns the required alignment in bytes for this component type.
    ///
    /// Crucial for custom raw buffers to ensure elements are written
    /// to addresses that are multiples of this alignment value.
    ///
    /// *Performance: $O(1)$ — Inline compile-time constant lookup.*
    #[inline]
    pub const fn align(&self) -> usize {
        self.layout.align()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::dealloc;
    use std::mem::forget;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use zoea_core::ecs::component::Component;

    // Global atomic counter tracking component destructor calls
    static COMPONENT_DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    // A heavy test component type with a strict explicit alignment requirement
    #[repr(align(16))]
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
            component_layout.align(),
            16,
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
