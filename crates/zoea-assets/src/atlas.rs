use std::collections::HashMap;
use zoea_core::rendering::assets::AssetId;
use zoea_core::rendering::uv_rectangle::UvRectangle;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpriteId(pub usize);
pub struct Atlas<'source> {
    pub source: &'source str,
    id: Option<AssetId>,
    next_sprite_id: usize,
    regions: HashMap<SpriteId, UvRectangle>,
    size: UvRectangle,
}

impl<'source> Atlas<'source> {
    pub fn new(source: &'source str, size: UvRectangle) -> Self {
        Self {
            source,
            id: None,
            next_sprite_id: 0,
            regions: HashMap::new(),
            size,
        }
    }

    pub fn register_atlas(&mut self, id: AssetId) {
        self.id = Some(id);
    }

    pub fn add_sprite(&mut self, x: u32, y: u32, w: u32, h: u32) -> SpriteId {
        let id = SpriteId(self.next_sprite_id);

        // Normalize coordinates: GPU likes 0.0 to 1.0
        let region = UvRectangle {
            x: x as f32 / self.size.width,
            y: y as f32 / self.size.height,
            width: w as f32 / self.size.width,
            height: h as f32 / self.size.height,
        };

        self.regions.insert(id, region);
        self.next_sprite_id += 1;

        id
    }

    pub fn set_id(&mut self, id: AssetId) {
        self.id = Some(id);
    }

    pub fn get_sprite(&self, id: SpriteId) -> Option<&UvRectangle> {
        self.regions.get(&id)
    }
}
