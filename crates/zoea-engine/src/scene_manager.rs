use crate::scene::Scene;
use std::collections::HashMap;


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SceneId(pub usize);


#[derive(Default)]
pub struct SceneManager {
    pub scenes: HashMap<SceneId, Scene>,
    pub current_scene_id: Option<SceneId>,
    next_scene_id: usize,
}

impl SceneManager {
    pub fn add_scene(&mut self, scene: Scene) -> SceneId {
        let id = SceneId(self.next_scene_id);

        self.scenes.insert(id, scene);
        self.next_scene_id += 1;

        id
    }

    pub fn select_scene(&mut self, scene_id: SceneId) {
        if let Some(current_scene_id) = self.current_scene_id {
            if let Some(current_scene) = self.scenes.get_mut(&current_scene_id) {
                current_scene.unload();
            }
        }

        if let Some(scene) = self.scenes.get_mut(&scene_id) {
            scene.load();
            self.current_scene_id = Some(scene_id);
        }
    }

    pub fn current_scene(&mut self) -> Option<&mut Scene> {
        let Some(current_scene_id) = self.current_scene_id else { return None };
        self.scenes.get_mut(&current_scene_id)
    }
}