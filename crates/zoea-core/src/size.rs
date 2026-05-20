use zoea_math::vec2::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Size(pub Vec2);

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self(Vec2::new(width, height))
    }

    pub fn zero() -> Self {
        Self(Vec2::zero())
    }

    pub fn width(&self) -> f32 {
        self.0.x
    }

    pub fn height(&self) -> f32 {
        self.0.y
    }
}

impl From<Size> for [f32; 2] {
    fn from(s: Size) -> Self {
        s.0.into()
    }
}
