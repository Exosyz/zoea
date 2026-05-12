use crate::math::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Position(pub Vec2);

impl From<Position> for [f32; 2] {
    fn from(p: Position) -> Self { p.0.into() }
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self(Vec2::new(x, y))
    }

    fn zero() -> Self { Self(Vec2::zero()) }
}

#[derive(Clone, Copy, Debug)]
pub struct Scale(pub Vec2);

impl Scale {
    pub fn new(x: f32, y: f32) -> Self {
        Self(Vec2::new(x, y))
    }

    fn zero() -> Self { Self(Vec2::zero()) }
}


impl From<Scale> for [f32; 2] {
    fn from(s: Scale) -> Self { s.0.into() }
}

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub position: Position,
    pub rotation: f32,
    pub scale: Scale,
}

impl Transform {
    pub fn new(position: Position, rotation: f32, scale: Scale) -> Self {
        Self { position, rotation, scale }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Position(Vec2::new(0.0, 0.0)),
            rotation: 0.0,
            scale: Scale(Vec2::new(1.0, 1.0)),
        }
    }
}