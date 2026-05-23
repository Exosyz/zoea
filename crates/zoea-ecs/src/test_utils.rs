use crate::storage::pending_component::PendingComponent;
use crate::topology::component_layout::ComponentLayout;
use crate::topology::component_registry::ComponentId;
use std::alloc::Layout;
use std::any::TypeId;
use std::ptr::{drop_in_place, NonNull};
use std::sync::{Arc, Mutex};
use zoea_core::ecs::component::Component;
use zoea_core::ecs::entity::EntityId;

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

pub(crate) unsafe fn drop_component_fn<T>(ptr: NonNull<u8>) {
    unsafe { drop_in_place(ptr.as_ptr() as *mut T) };
}

pub(crate) unsafe fn drop_noop_fn(_ptr: NonNull<u8>) {}

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

pub(crate) fn create_layout<T: Component>(drop_fn: unsafe fn(NonNull<u8>)) -> ComponentLayout {
    ComponentLayout {
        id: get_id::<T>(),
        layout: Layout::new::<T>(),
        drop_fn,
    }
}

pub(crate) fn create_component<T: Component>(component: T) -> PendingComponent {
    PendingComponent::new::<T>(get_id::<T>(), component)
}

pub(crate) fn create_entity(id: u32) -> EntityId {
    EntityId::new(id, 1)
}
