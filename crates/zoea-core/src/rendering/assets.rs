use crate::rendering::uv_rectangle::UvRectangle;
use crate::transform::Transform;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetId(pub usize);

#[derive(Copy, Clone, Debug)]
pub struct Sprite {
    pub id: AssetId,
    pub uv_rectangle: UvRectangle,
}

#[derive(Copy, Clone, Debug)]
pub struct DrawInstance {
    pub uv_rectangle: UvRectangle,
    pub transform: Transform,
}

impl From<(Transform, UvRectangle)> for DrawInstance {
    fn from((transform, uv_rectangle): (Transform, UvRectangle)) -> Self {
        DrawInstance {
            transform,
            uv_rectangle,
        }
    }
}
