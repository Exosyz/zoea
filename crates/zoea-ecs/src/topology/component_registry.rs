use crate::error::EcsError;
use std::any::TypeId;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{OnceLock, RwLock};
use zoea_core::ecs::component::Component;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ComponentId(pub usize);

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
static REGISTRY: OnceLock<RwLock<HashMap<TypeId, ComponentId>>> = OnceLock::new();

/// Maps a Rust type globally and thread-safely to a unique `ComponentId`.
///
/// This registry is optimized for high-performance ECS architectures where
/// sequential, dense IDs are needed to index into component bitmasks or sparse sets.
///
/// ### Thread Safety
/// Access is coordinated using a global `OnceLock` protecting an `RwLock`.
/// - **Read Path (Fast Path):** Uses a read-guard allowing multiple threads to parallel-query
///   existing component IDs with near-zero contention during execution loops.
/// - **Write Path (Slow Path):** Uses a write-guard paired with entry tracking to ensure
///   idempotent, race-free ID allocation if multiple threads attempt to register a new type simultaneously.
///
/// ### Errors
/// Returns an `Err(EcsError::ComponentLimitExceeded)` if the total number of unique registered
/// components attempts to cross the hard limit of 256.
pub fn get_component_id<T: Component + 'static>() -> Result<ComponentId, EcsError> {
    let registry_lock = REGISTRY.get_or_init(|| RwLock::new(HashMap::new()));

    {
        let read_guard = registry_lock.read().unwrap();
        if let Some(&id) = read_guard.get(&TypeId::of::<T>()) {
            return Ok(id);
        }
    }

    let mut write_guard = registry_lock.write().unwrap();

    match write_guard.entry(TypeId::of::<T>()) {
        Entry::Occupied(occupied_entry) => Ok(*occupied_entry.get()),
        Entry::Vacant(vacant_entry) => {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            if id >= 256 {
                return Err(EcsError::ComponentLimitExceeded);
            }
            Ok(*vacant_entry.insert(ComponentId(id)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Standard structural mock components
    struct TestCompA;
    impl Component for TestCompA {}

    struct TestCompB;
    impl Component for TestCompB {}

    #[test]
    fn test_id_uniqueness_and_consistency() {
        let id1 = get_component_id::<TestCompA>().unwrap();
        let id2 = get_component_id::<TestCompA>().unwrap();
        let id3 = get_component_id::<TestCompB>().unwrap();

        assert_eq!(
            id1, id2,
            "Subsequent calls for the same type must return the same ID."
        );
        assert_ne!(id1, id3, "Different types must receive unique IDs.");
    }

    #[test]
    fn test_thread_safety_parallel_access() {
        use std::thread;

        let mut handles = Vec::new();
        for _ in 0..16 {
            handles.push(thread::spawn(|| get_component_id::<TestCompA>().unwrap()));
        }

        for handle in handles {
            let id = handle.join().unwrap();
            assert_eq!(
                id,
                get_component_id::<TestCompA>().unwrap(),
                "Threaded initialization broke ID consistency."
            );
        }
    }

    #[test]
    fn test_component_limit_exceeded() {
        // To test the 256 limit without manually declaring 256 structs,
        // we can fast-forward the underlying atomic counter safely.
        // We capture the original state first so we don't break other tests permanently.
        let original_id = NEXT_ID.load(Ordering::SeqCst);

        // Force atomic to limit threshold
        NEXT_ID.store(256, Ordering::SeqCst);

        // Define a brand new un-registered component type
        struct OverflownComponent;
        impl Component for OverflownComponent {}

        let result = get_component_id::<OverflownComponent>();

        // Restore the original ID so other tests running in the process don't break
        NEXT_ID.store(original_id, Ordering::SeqCst);

        assert!(
            matches!(result, Err(EcsError::ComponentLimitExceeded)),
            "Expected Err(EcsError::ComponentLimitExceeded) when counter >= 256, got {:?}",
            result
        );
    }
}
