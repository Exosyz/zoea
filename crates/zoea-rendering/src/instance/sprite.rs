use zoea_core::rendering::assets::DrawInstance;

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

impl From<&DrawInstance> for SpriteInstance {
    fn from(instance: &DrawInstance) -> Self {
        Self {
            position: instance.transform.position.into(),
            scale: instance.transform.scale.into(),
            uv_offset: [instance.uv_rectangle.x, instance.uv_rectangle.y],
            uv_scale: [instance.uv_rectangle.width, instance.uv_rectangle.height],
            rotation: instance.transform.rotation.into(),
            _padding: [0.0; 3],
        }
    }
}
