use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
pub enum Rotation {
    Radian(f32),
    Degrees(f32),
}

impl Into<f32> for Rotation {
    fn into(self) -> f32 {
        match self {
            Rotation::Radian(radian) => { radian }
            Rotation::Degrees(degree) => {
                degree * PI / 180.0
            }
        }
    }
}