use crate::error::EcsError;
use crate::storage::pending_component::PendingComponent;
use crate::topology::component_registry::get_component_id;
use crate::world::World;
use zoea_core::ecs::component::Component;
use zoea_core::ecs::entity::EntityId;

/// A fluent factory pattern builder responsible for staging, validating,
/// and committing new entities with their associated components into dense archetype storage.
///
/// `EntityBuilder` acts as a transactional staging area. It heap-allocates incoming
/// components temporarily, cleans them up safely if an operation fails or is abandoned,
/// and copies them into linear chunk arrays upon a successful build step.
///
/// ### Safety Invariants
/// * **Transactional Integrity**: If a build fails midway or is explicitly dropped,
///   standard RAII drops the staging vector, triggering `PendingComponent` destructors
///   to prevent memory leaks.
/// * **Memory Hand-off**: On a successful `.build()`, the underlying pointers are stripped
///   of their ownership capabilities using `release_allocation_shell()` so that only
///   the archetype chunk retains dropping responsibility.
pub struct EntityBuilder<'world> {
    world: &'world mut World,
    components: Vec<PendingComponent>,
}

impl<'world> EntityBuilder<'world> {
    /// Creates a new `EntityBuilder` bound to the lifecycle of the given mutable [`World`] reference.
    ///
    /// Initializes with an internal staging capacity of 8 components to minimize reallocations.
    pub fn new(world: &'world mut World) -> Self {
        Self {
            world,
            components: Vec::with_capacity(8),
        }
    }

    /// Enqueues a component into the temporary type-erased staging buffer.
    ///
    /// The component is immediately moved to the heap to yield a stable, type-erased raw pointer.
    ///
    /// # Errors
    ///
    /// Returns an [`EcsError`] if the component type `T` has not been registered in the system
    /// registry or lacks a valid runtime component ID assignment.
    pub fn try_add<T>(mut self, component: T) -> Result<Self, EcsError>
    where
        T: Component,
    {
        let id = get_component_id::<T>()?;

        self.components.push(PendingComponent::new(id, component));

        Ok(self)
    }

    /// Sorts the staged components by runtime ID, resolves the destination archetype,
    /// copies the component payloads into chunk storage, and registers the newly minted [`EntityId`].
    ///
    /// # Errors
    ///
    /// Returns an [`EcsError`] if:
    /// * An archetype layout cannot be allocated or matched for this component grouping.
    /// * The system fails to acquire mutable access to the target archetype.
    /// * Target chunk allocations fail to accept the entity layout.
    /// * `EcsError::DuplicateComponent` if the same component type is added multiple times.
    ///
    /// # Safety
    ///
    /// This method performs bitwise layout copies from raw pointers into component arrays.
    /// To ensure memory safety, the builder hands over ownership of the staging vector to
    /// the archetype, which releases allocation shells as soon as injection succeeds.
    pub fn build(mut self) -> Result<EntityId, EcsError> {
        self.components.sort_by_key(|c| c.id);

        for windows in self.components.windows(2) {
            if windows[0].id == windows[1].id {
                return Err(EcsError::DuplicateComponent);
            }
        }

        let archetype_id = self.world.get_or_create_archetype(&self.components)?;
        let entity_id = self.world.generate_entity_id();
        let archetype = self.world.get_archetype_mut(archetype_id)?;

        let entity_location = archetype.spawn(entity_id, self.components)?;

        self.world
            .insert_entity_location(entity_id, entity_location);

        Ok(entity_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    // A heavy heap component designed to track raw drop execution behavior
    #[derive(Clone)]
    struct TrackedComponent {
        drop_counter: Arc<AtomicUsize>,
        _payload: String, // Force heap allocations to trigger Miri if double dropped
    }

    impl Component for TrackedComponent {}

    impl Drop for TrackedComponent {
        fn drop(&mut self) {
            self.drop_counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_builder_lifecycle_does_not_drop_payload_on_successful_build() {
        let drop_counter = Arc::new(AtomicUsize::new(0));

        // 1. Initialize context scope
        let mut world = World::new();
        let component_instance = TrackedComponent {
            drop_counter: drop_counter.clone(),
            _payload: "Zoea Performance Payload String Core".to_string(),
        };

        // 2. Run execution pipeline
        let builder = EntityBuilder::new(&mut world);
        let _entity = builder
            .try_add(component_instance)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(
            drop_counter.load(Ordering::SeqCst),
            0,
            "The builder erroneously dropped the component payload during release extraction!"
        );
    }

    #[test]
    fn test_builder_unwinding_drops_payload_on_builder_abandonment() {
        let drop_counter = Arc::new(AtomicUsize::new(0));
        let mut world = World::new();

        {
            let builder = EntityBuilder::new(&mut world);
            let context_component = TrackedComponent {
                drop_counter: drop_counter.clone(),
                _payload: "Temporary Drop Verification Payload".to_string(),
            };

            let _unbuilt_builder = builder.try_add(context_component).unwrap();
            // Builder is intentionally dropped out of scope here without invoking .build()
        }

        assert_eq!(
            drop_counter.load(Ordering::SeqCst),
            1,
            "PendingComponent failed to drop heap resources when the builder was abandoned!"
        );
    }

    #[test]
    fn test_builder_prevents_duplicate_components() {
        let drop_counter = Arc::new(AtomicUsize::new(0));
        let mut world = World::new();

        let c1 = TrackedComponent {
            drop_counter: drop_counter.clone(),
            _payload: "First instance".to_string(),
        };
        let c2 = TrackedComponent {
            drop_counter: drop_counter.clone(),
            _payload: "Second duplicate instance".to_string(),
        };

        let result = EntityBuilder::new(&mut world)
            .try_add(c1)
            .unwrap()
            .try_add(c2)
            .unwrap()
            .build();

        assert!(matches!(result, Err(EcsError::DuplicateComponent)));

        // Transactional integrity check: Both components should be safely dropped on failure
        assert_eq!(drop_counter.load(Ordering::SeqCst), 2);
    }
}
