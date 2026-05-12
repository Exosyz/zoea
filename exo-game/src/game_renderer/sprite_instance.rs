use crate::game_renderer::atlas::SpriteRegion;
use crate::transform::Transform;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    // 1. The Transform (Where it is)
    pub position: [f32; 2],
    pub scale: [f32; 2],

    // 2. The Atlas Data (What it looks like)
    pub uv_offset: [f32; 2],
    pub uv_scale: [f32; 2],
    pub rotation: f32,  // In Radians
    _padding: [f32; 3], // 3 * 4 = 12 bytes. Total struct = 48 bytes.
}

impl SpriteInstance {
    pub fn new(transform: &Transform, sprite_region: &SpriteRegion) -> Self {
        Self {
            position: transform.position.into(),
            scale: transform.scale.into(),
            uv_offset: [sprite_region.x, sprite_region.y],
            uv_scale: [sprite_region.width, sprite_region.height],
            rotation: transform.rotation,
            _padding: [0.0; 3],
        }
    }
}
