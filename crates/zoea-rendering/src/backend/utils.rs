use crate::assets_manager::AssetManager;
use std::sync::Arc;
use wgpu::{
    BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState,
    ColorTargetState, ColorWrites, Device, Extent3d, FragmentState, FrontFace, MultisampleState,
    Origin3d, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, Queue,
    RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, SurfaceConfiguration, TexelCopyBufferLayout, TexelCopyTextureInfo,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexState,
};

/// 1. Texture Layout Factory
/// Creates a standard (Texture + Sampler) layout used by almost every sprite shader.
pub fn create_texture_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Texture + Sampler Layout"),
        entries: &[
            // The Texture (Binding 0)
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
            // The Sampler (Binding 1)
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// 3. Texture Asset Factory
/// Handles the heavy lifting of uploading raw RGBA bytes to the GPU.
/// In production, this would also handle Mipmap generation.
pub fn upload_texture_to_gpu(
    device: &Device,
    queue: &Queue,
    rgba: &[u8],
    dimension: (u32, u32),
    label: &str,
) -> TextureView {
    let size = Extent3d {
        width: dimension.0,
        height: dimension.1,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        rgba,
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * dimension.0),
            rows_per_image: Some(dimension.1),
        },
        size,
    );

    texture.create_view(&TextureViewDescriptor::default())
}

pub fn build_rendering_pipeline(
    device: Arc<Device>,
    config: &SurfaceConfiguration,
    asset_manager: &AssetManager,
) -> RenderPipeline {
    // 1. Create the Pipeline Layout
    // This defines the "interface" between your Rust code and the Shader.
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Sprite Pipeline Layout"),
        bind_group_layouts: &[
            Some(&asset_manager.layout),
            Some(&asset_manager.instances.layout),
            Some(&asset_manager.camera.layout),
        ],
        immediate_size: 0,
    });

    // 2. Load the Shader
    // include_str! embeds the file into your binary at compile time.
    let shader_source = include_str!("../../assets/atlas_rendering_shader.wgsl");
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Atlas Shader"),
        source: ShaderSource::Wgsl(shader_source.into()),
    });

    // 3. Build the actual Pipeline
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Sprite Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: config.format,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
