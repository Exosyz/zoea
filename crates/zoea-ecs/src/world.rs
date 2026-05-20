use zoea_core::rendering::assets::Sprite;
use zoea_core::transform::Transform;

pub struct TempEntity {
    pub transform: Transform,
    pub sprite: Sprite,
}
#[derive(Default)]
pub struct World {
    entities: Vec<TempEntity>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entity(&mut self, entity: TempEntity) {
        self.entities.push(entity);
    }

    pub fn entities(&self) -> &[TempEntity] {
        &self.entities
    }

    pub fn clear(&mut self) {
        self.entities.clear();
    }
}
