use crate::math::Vec2;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // A 4x4 Matrix represented as an array
    pub view_proj: [[f32; 4]; 4],
}

pub struct Camera(pub Vec2);
pub struct Size(pub Vec2);

impl CameraUniform {
    pub fn from_size(width: u32, height: u32) -> Self {
        Self::new(
            Size(Vec2::new(width as f32, height as f32)),
            Camera(Vec2::new(0.0, 0.0)),
            1.0,
        )
    }

    fn new(size: Size, camera: Camera, zoom: f32) -> Self {
        let w = size.0.x;
        let h = size.0.y;

        let scale_x = (2.0 / w) * zoom;
        let scale_y = (-2.0 / h) * zoom;

        let tx = -(camera.0.x * scale_x);
        let ty = -(camera.0.y * scale_y);

        Self {
            view_proj: [
                [scale_x, 0.0, 0.0, 0.0],
                [0.0, scale_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-1.0 + tx, 1.0 + ty, 0.0, 1.0],
            ],
        }
    }
}
