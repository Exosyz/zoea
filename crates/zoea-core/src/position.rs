use zoea_math::vec2::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Position(pub Vec2);

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self(Vec2::new(x, y))
    }

    pub fn zero() -> Self {
        Self(Vec2::zero())
    }

    pub fn x(&self) -> f32 {
        self.0.x
    }

    pub fn y(&self) -> f32 {
        self.0.y
    }
}

impl From<Position> for [f32; 2] {
    fn from(p: Position) -> Self {
        p.0.into()
    }
}
