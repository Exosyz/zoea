use crate::camera::CameraUniform;
use crate::game_renderer::atlas::Atlas;
use crate::game_renderer::gpu_resource::{GpuResource, GpuResourceMode};
use crate::game_renderer::samplers::Samplers;
use crate::game_renderer::sprite_instance::SpriteInstance;
use crate::game_renderer::wgpu_utils::{
    create_texture_bind_group_layout, upload_texture_to_gpu,
};
use image::GenericImageView;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout
    , BindingResource, Device,
    Queue, ShaderStages,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AtlasId(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AssetError {
    FileNotFound,
    InvalidFormat,
}

pub struct AssetManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    next_atlas_id: usize,
    atlas: HashMap<AtlasId, BindGroup>,
    atlas_metadata: HashMap<AtlasId, Atlas>,
    atlas_name: HashMap<String, AtlasId>,
    pub layout: BindGroupLayout,
    pub instances: GpuResource<SpriteInstance>,
    pub camera: GpuResource<CameraUniform>,
    samplers: Samplers,
}

impl AssetManager {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let layout = create_texture_bind_group_layout(&device);

        let samplers = Samplers::new(&device);

        let instances = GpuResource::new(
            device.clone(),
            "Instances",
            0,
            ShaderStages::VERTEX,
            GpuResourceMode::Storage(1000),
        );
        let camera = GpuResource::new(
            device.clone(),
            "Camera",
            0,
            ShaderStages::VERTEX,
            GpuResourceMode::Uniform,
        );

        Self {
            device,
            queue,
            next_atlas_id: 0,
            atlas: HashMap::new(),
            atlas_name: HashMap::new(),
            atlas_metadata: HashMap::new(),
            layout,
            instances,
            camera,
            samplers,
        }
    }

    pub fn load(
        &mut self,
        source: &str,
        width: u32,
        height: u32,
        mut setup_atlas: impl FnMut(&mut Atlas),
    ) -> Result<AtlasId, AssetError> {
        if let Some(&id) = self.atlas_name.get(source) {
            return Ok(id);
        }

        let mut loaded_atlas = Atlas::new(source, width, height);

        setup_atlas(&mut loaded_atlas);

        let bytes = fs::read(source).or(Err(AssetError::FileNotFound))?;
        let image =
            image::load_from_memory(bytes.as_slice()).map_err(|_| AssetError::InvalidFormat)?;
        let rgba = image.to_rgba8();
        let dimensions = image.dimensions();

        let texture_view =
            upload_texture_to_gpu(&self.device, &self.queue, &rgba, dimensions, source);

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            layout: &self.layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.samplers.nearest_clamp),
                },
            ],
            label: None,
        });

        let id = AtlasId(self.next_atlas_id);
        self.next_atlas_id += 1;

        self.atlas.insert(id, bind_group);
        self.atlas_metadata.insert(id, loaded_atlas);
        self.atlas_name.insert(source.into(), id);

        Ok(id)
    }

    pub fn unload(&mut self, id: AtlasId) {
        unimplemented!()
    }

    pub fn get(&self, id: AtlasId) -> Option<&BindGroup> {
        self.atlas.get(&id)
    }

    pub fn get_metadata(&self, id: AtlasId) -> Option<&Atlas> {
        self.atlas_metadata.get(&id)
    }
}
