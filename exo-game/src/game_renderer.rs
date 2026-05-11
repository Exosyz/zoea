use crate::assets_manager::AssetManager;
use std::sync::Arc;
use std::{env, fs};
use wgpu::hal::SurfaceError;
use wgpu::{
    Backends, BlendState, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, Device,
    DeviceDescriptor, FragmentState, Instance, LoadOp, MultisampleState, Operations,
    PipelineLayoutDescriptor, PresentMode, PrimitiveState, PrimitiveTopology, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    RequestAdapterOptions, ShaderModuleDescriptor, ShaderSource, StoreOp, Surface,
    SurfaceConfiguration, TextureUsages, TextureViewDescriptor, VertexState,
};
use winit::window::Window;

pub struct GameRenderer<'window> {
    surface: Surface<'window>,
    config: SurfaceConfiguration,
    device: Arc<Device>,
    queue: Arc<Queue>,
    pub(crate) window: Arc<Window>,
    pub(crate) assets_manager: AssetManager,
    render_pipeline: RenderPipeline,
}

impl<'window> GameRenderer<'window> {
    pub async fn new(window: Arc<Window>, shader_path: &str) -> Self {
        let instance = Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .expect("Surface creation failed");

        // 1. Get all adapters asynchronously
        let all_adapters = instance.enumerate_adapters(Backends::all()).await;

        // 2. Find a compatible one or fallback
        let adapter = all_adapters
            .into_iter()
            .find(|ad| {
                let caps = surface.get_capabilities(ad);
                !caps.formats.is_empty()
            })
            .or_else(|| {
                pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
                    force_fallback_adapter: true,
                    ..Default::default()
                }))
                .ok()
            })
            .expect("No compatible adapter found. Check WSLg/Mesa drivers.");

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default())
            .await
            .expect("Failed to create device");

        let caps = surface.get_capabilities(&adapter);
        let size = window.inner_size();

        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        println!(
            "Configuring surface with format: {:?} and Alpha: {:?}",
            surface_format, config.alpha_mode
        );
        surface.configure(&device, &config);

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let assets_manager = AssetManager::new(device.clone(), queue.clone());
        dbg!(shader_path);
        dbg!(env::current_dir());
        let shader_file_content = fs::read_to_string(shader_path).expect("Shader not found");

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(shader_file_content.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Sprite Pipeline Layout"),
            bind_group_layouts: &[Some(&assets_manager.layout)],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Sprite Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[], // Empty because we are hardcoding the square in the shader
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    // Enable transparency (Blending)
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList, // 3 vertices = 1 tri
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            surface,
            config,
            device,
            queue,
            window,
            assets_manager,
            render_pipeline,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn draw(&mut self) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Main draw rendering"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::GREEN),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);

            // TODO Call ECS to retrieve sprites and draw them
            if let Some(bind_group) = self.assets_manager.get(0) {
                render_pass.set_bind_group(0, bind_group, &[]);

                render_pass.draw(0..6, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    pub fn render(&mut self, interpolation: f32) -> Result<(), SurfaceError> {
        dbg!("render");
        self.draw();
        Ok(())
    }
}
