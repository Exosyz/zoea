use crate::samplers::Samplers;
use image::GenericImageView;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Device, Extent3d, Origin3d, Queue,
    SamplerBindingType, ShaderStages, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};

pub type AssetId = usize;

pub enum AssetError {
    FileNotFound,
    InvalidFormat,
}

pub struct AssetManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    next_asset_id: AssetId,
    assets: HashMap<AssetId, BindGroup>,
    assets_name: HashMap<String, AssetId>,
    pub layout: BindGroupLayout,
    samplers: Samplers,
}

impl AssetManager {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("bind_group_layout"),
        });

        let samplers = Samplers::new(&device);

        Self {
            device,
            queue,
            next_asset_id: 0,
            assets: HashMap::new(),
            assets_name: HashMap::new(),
            layout: bind_group_layout,
            samplers,
        }
    }

    pub fn load(&mut self, source: &str) -> Result<AssetId, AssetError> {
        if let Some(&id) = self.assets_name.get(source) {
            return Ok(id);
        }

        let bytes = fs::read(source).or(Err(AssetError::FileNotFound))?;
        let image =
            image::load_from_memory(bytes.as_slice()).map_err(|_| AssetError::InvalidFormat)?;
        let rgba = image.to_rgba8();
        let dimensions = image.dimensions();

        let texture_size = Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            label: Some(source),
            view_formats: &[],
        });

        self.queue.write_texture(
            TexelCopyTextureInfo {
                // Use ImageCopyTexture if on older wgpu
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &rgba,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            layout: &self.layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.samplers.linear_clamp),
                },
            ],
            label: Some(&format!("{}_bind_group", source)),
        });

        let id = self.next_asset_id;
        self.next_asset_id += 1;

        self.assets.insert(id, bind_group);
        self.assets_name.insert(source.into(), id);

        Ok(id)
    }

    pub fn get(&self, id: AssetId) -> Option<&BindGroup> {
        self.assets.get(&id)
    }
}
