use crate::ecs::component::Component;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct UvRectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl UvRectangle {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_size(width: f32, height: f32) -> Self {
        Self::new(0.0, 0.0, width, height)
    }
}

impl Component for UvRectangle {
    const TARGET_GPU: bool = true;
}
