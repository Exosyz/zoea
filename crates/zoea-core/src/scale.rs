use zoea_math::vec2::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Scale(pub Vec2);

impl Scale {
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

impl From<Scale> for [f32; 2] {
    fn from(s: Scale) -> Self {
        s.0.into()
    }
}

impl From<f32> for Scale {
    fn from(s: f32) -> Self {
        Self::new(s, s)
    }
}
