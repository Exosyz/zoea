use zoea_core::size::Size;
use zoea_core::transform::Position;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraInstance {
    // A 4x4 Matrix represented as an array
    pub view_proj: [[f32; 4]; 4],
}

impl CameraInstance {
    pub fn from_size(size: Size) -> Self {
        Self::new(size, Position::new(0.0, 0.0), 1.0)
    }

    fn new(size: Size, position: Position, zoom: f32) -> Self {
        let scale_x = (2.0 / size.width()) * zoom;
        let scale_y = (-2.0 / size.height()) * zoom;

        let tx = -(position.x() * scale_x);
        let ty = -(position.y() * scale_y);

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
