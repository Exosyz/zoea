use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SpriteRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpriteId(pub usize);
pub struct Atlas {
    file_name: String,
    next_sprite_id: usize,
    regions: HashMap<SpriteId, SpriteRegion>,
    width: u32,
    height: u32,
}

impl Atlas {
    pub fn new(file_name: &str, width: u32, height: u32) -> Self {
        Self {
            file_name: file_name.to_string(),
            next_sprite_id: 0,
            regions: HashMap::new(),
            width,
            height,
        }
    }

    pub fn add_sprite(&mut self, x: u32, y: u32, w: u32, h: u32) -> &mut Self {
        let id = self.next_sprite_id;

        // Normalize coordinates: GPU likes 0.0 to 1.0
        let region = SpriteRegion {
            x: x as f32 / self.width as f32,
            y: y as f32 / self.height as f32,
            width: w as f32 / self.width as f32,
            height: h as f32 / self.height as f32,
        };

        self.regions.insert(SpriteId(id), region);
        self.next_sprite_id += 1;

        self
    }

    pub fn get_sprite(&self, id: SpriteId) -> Option<&SpriteRegion> {
        self.regions.get(&id)
    }
}
