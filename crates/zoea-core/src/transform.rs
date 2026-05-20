pub use crate::position::Position;
use crate::rotation::Rotation;
pub use crate::scale::Scale;
use zoea_math::vec2::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub position: Position,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Transform {
    pub fn new(position: Position, rotation: Rotation, scale: Scale) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Position(Vec2::new(0.0, 0.0)),
            rotation: Rotation::Radian(0.0),
            scale: Scale(Vec2::new(1.0, 1.0)),
        }
    }
}

impl From<Position> for Transform {
    fn from(position: Position) -> Self {
        Self {
            position,
            rotation: Rotation::Radian(0.0),
            scale: Scale(Vec2::new(100.0, 100.0)),
        }
    }
}