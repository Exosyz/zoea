//! Test infrastructure utilities and mock components for ECS behavior validation.
//!
//! Provides mock component types (`Position`, `Velocity`, `DroppableComponent`),
//! type-to-id mapping fixtures, and deterministic manual entity injection wrappers
//! to bypass the full builder lifecycle during isolation tests.

use crate::storage::pending_component::PendingComponent;
use crate::topology::component_layout::ComponentLayout;
use crate::topology::component_registry::ComponentId;
use crate::world::World;
use std::alloc::Layout;
use std::any::TypeId;
use std::ptr::{drop_in_place, NonNull};
use std::sync::{Arc, Mutex};
use zoea_core::ecs::component::Component;
use zoea_core::ecs::entity::EntityId;

/// Mock 2D spatial coordinate component.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub(crate) struct Position {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

impl From<i32> for Position {
    fn from(value: i32) -> Self {
        Position { x: value, y: value }
    }
}

impl Component for Position {}

/// Mock 2D velocity component.
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub(crate) struct Velocity {
    pub(crate) dx: f32,
    pub(crate) dy: f32,
}

impl From<f32> for Velocity {
    fn from(value: f32) -> Self {
        Velocity {
            dx: value,
            dy: value,
        }
    }
}

impl Component for Velocity {}

/// Staging component equipped with an atomic/mutex counter to verify drop tracking.
#[derive(Clone)]
pub(crate) struct DroppableComponent {
    pub(crate) counter: Arc<Mutex<usize>>,
}

impl Component for DroppableComponent {}

impl Drop for DroppableComponent {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.counter.lock() {
            *guard += 1;
        }
    }
}

/// Dynamic type-erased helper hook used to cleanly invoke `Drop::drop` implementations.
pub(crate) unsafe fn drop_component_fn<T>(ptr: NonNull<u8>) {
    unsafe { drop_in_place(ptr.as_ptr() as *mut T) };
}

/// No-op destructor placeholder for trivial or copy-native types.
pub(crate) unsafe fn drop_noop_fn(_ptr: NonNull<u8>) {}

/// Returns a deterministic fallback mock component ID for isolated unit tests.
pub(crate) fn get_id<T: Component>() -> ComponentId {
    let t_id = TypeId::of::<T>();

    if t_id == TypeId::of::<Position>() {
        ComponentId(0)
    } else if t_id == TypeId::of::<Velocity>() {
        ComponentId(1)
    } else if t_id == TypeId::of::<DroppableComponent>() {
        ComponentId(2)
    } else {
        ComponentId(3)
    }
}

/// Instantiates a raw `ComponentLayout` metadata wrapper using mock behaviors.
pub(crate) fn create_layout<T: Component>(drop_fn: unsafe fn(NonNull<u8>)) -> ComponentLayout {
    ComponentLayout {
        id: get_id::<T>(),
        layout: Layout::new::<T>(),
        drop_fn,
    }
}

/// Helper method to wrap an instance of `T` into a mock `PendingComponent` staging block.
pub(crate) fn create_component<T: Component>(component: T) -> PendingComponent {
    PendingComponent::new::<T>(get_id::<T>(), component)
}

/// Instantiates a new test `EntityId` wrapper with a fixed generation sequence of 1.
pub(crate) fn create_entity(id: u32) -> EntityId {
    EntityId::new(id, 1)
}

/// Grabs a raw type-erased pointer out of a reference to an `EntityId`.
pub(crate) fn get_entity_id_ptr(id: &mut EntityId) -> NonNull<u8> {
    NonNull::new(id as *mut _ as *mut u8).unwrap()
}

/// Spawns an empty entity manually inside the world's internal archetypes for isolation testing.
///
/// Bypasses the `EntityBuilder` pipeline to evaluate underlying storage behavior directly.
pub(crate) fn spawn_empty_entity(world: &mut World, raw_id: u32) -> EntityId {
    let mut entity_id = create_entity(raw_id);

    // Resolve or allocate the empty-component archetype layout layout ([])
    let arch_id = world.get_or_create_archetype(&vec![]).unwrap();
    let archetype = world.get_archetype_mut(arch_id).unwrap();

    // Inject the raw stack reference directly; underlying architecture performs immediate copy bitwise
    let ptrs = vec![NonNull::new(&mut entity_id as *mut _ as *mut u8).unwrap()];
    let loc = archetype.inject_entity(entity_id, ptrs).unwrap();

    world.insert_entity_location(entity_id, loc);

    entity_id
}
