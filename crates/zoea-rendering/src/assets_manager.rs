use crate::backend::resource::{GpuResource, GpuResourceMode};
use crate::backend::samplers::Samplers;
use crate::backend::utils::{create_texture_bind_group_layout, upload_texture_to_gpu};
use crate::instance::camera::CameraInstance;
use crate::instance::sprite::SpriteInstance;
use image::GenericImageView;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Device,
    Queue, ShaderStages,
};
use zoea_core::rendering::assets::AssetId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AssetError {
    FileNotFound,
    InvalidFormat,
}

pub struct AssetManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    next_asset_id: usize,
    assets: HashMap<AssetId, BindGroup>,
    asset_names: HashMap<String, AssetId>,
    pub layout: BindGroupLayout,
    pub instances: GpuResource<SpriteInstance>,
    pub camera: GpuResource<CameraInstance>,
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
            next_asset_id: 0,
            assets: HashMap::new(),
            asset_names: HashMap::new(),
            layout,
            instances,
            camera,
            samplers,
        }
    }

    pub fn load(&mut self, source: &str) -> Result<AssetId, AssetError> {
        if let Some(&id) = self.asset_names.get(source) {
            return Ok(id);
        }

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

        let id = AssetId(self.next_asset_id);
        self.next_asset_id += 1;

        self.assets.insert(id, bind_group);
        self.asset_names.insert(source.into(), id);

        Ok(id)
    }

    pub fn unload(&mut self, _id: AssetId) {
        unimplemented!()
    }

    pub fn get(&self, id: AssetId) -> Option<&BindGroup> {
        self.assets.get(&id)
    }
}
