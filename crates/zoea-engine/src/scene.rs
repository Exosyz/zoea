use crate::game_logic::GameLogic;
use std::collections::HashMap;
use zoea_core::rendering::assets::DrawInstance;
use zoea_ecs::world::{TempEntity, World};
use zoea_rendering::renderer::GameRenderer;

#[derive(Default)]
pub struct Scene {
    world: World,
    game_logic: GameLogic,
}

impl Scene {
    pub fn add_entities(&mut self, entities: Vec<TempEntity>) {
        entities.into_iter().for_each(|entity| {
            self.world.add_entity(entity);
        })
    }
    pub fn load(&mut self) {}

    pub fn unload(&mut self) {
        // self.world.clear();
    }

    pub fn render(&mut self, renderer: &mut GameRenderer, _interpolation: f32) {
        let mut instances = HashMap::new();
        for entity in self.world.entities() {
            if !instances.contains_key(&entity.sprite.id) {
                instances.insert(entity.sprite.id, vec![]);
            }

            let array = instances.get_mut(&entity.sprite.id).expect(format!("Sprite id {:?} need to exist ",
                                                                            &entity.sprite.id).as_str());

            array.push(DrawInstance::from((
                entity.transform,
                entity.sprite.uv_rectangle,
            )));
        }

        renderer.draw(instances);
    }

    pub fn tick(&mut self) {
        self.game_logic.update()
    }
}
