use crate::camera::CameraUniform;
use crate::game_renderer::assets_manager::AtlasId;
use crate::game_renderer::atlas::SpriteId;
use crate::game_renderer::sprite_instance::SpriteInstance;
use crate::game_renderer::wgpu_utils::build_rendering_pipeline;
use crate::transform::{Position, Scale, Transform};
use assets_manager::AssetManager;
use std::sync::Arc;
use wgpu::hal::SurfaceError;
use wgpu::{
    Adapter, Color, CommandEncoderDescriptor, CurrentSurfaceTexture, Device, DeviceDescriptor,
    Instance, LoadOp, Operations, PresentMode, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RequestAdapterOptions, StoreOp, Surface,
    SurfaceCapabilities, SurfaceConfiguration, TextureUsages, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

mod assets_manager;
mod atlas;
mod gpu_resource;
pub mod samplers;
mod sprite_instance;
mod wgpu_utils;

pub struct GameRenderer<'window> {
    surface: Surface<'window>,
    config: SurfaceConfiguration,
    device: Arc<Device>,
    queue: Arc<Queue>,
    pub(crate) window: Arc<Window>,
    pub(crate) assets_manager: AssetManager,
    render_pipeline: RenderPipeline,
    camera: CameraUniform,
}

impl<'window> GameRenderer<'window> {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .expect("Surface failed");

        let adapter = Self::select_adapter(&instance, &surface).await;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default())
            .await
            .expect("Device failed");

        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let caps = surface.get_capabilities(&adapter);
        let config = Self::create_surface_config(window.inner_size(), &caps);

        surface.configure(&device, &config);

        let assets_manager = AssetManager::new(device.clone(), queue.clone());
        let render_pipeline = build_rendering_pipeline(device.clone(), &config, &assets_manager);

        let camera = CameraUniform::from_size(config.width, config.height);

        Self {
            surface,
            config,
            device,
            queue,
            window,
            assets_manager,
            render_pipeline,
            camera,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.camera = CameraUniform::from_size(new_size.width, new_size.height);
        }
    }

    pub fn draw(&mut self, _interpolation: f32) {
        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(f) => f,
            CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };

        self.assets_manager
            .camera
            .write_single(&self.queue, &self.camera);

        // 1. Resolve Data (Flattened)
        // In a real loop, we'd iterate over entities, but for this test:
        let atlas_id = AtlasId(0);
        let sprite_id = SpriteId(0);

        let Some(atlas_bind_group) = self.assets_manager.get(atlas_id) else {
            return;
        };
        let Some(meta) = self.assets_manager.get_metadata(atlas_id) else {
            return;
        };
        let Some(region) = meta.get_sprite(sprite_id) else {
            return;
        };

        let transforms = [
            Transform::new(Position::new(10.0, 10.0), 0.0, Scale::new(100.0, 100.0)),
            Transform::new(Position::new(100.0, 100.0), 0.0, Scale::new(50.0, 50.0)),
            //Transform::new(Position::new(10.0, 1000.0), 1.0, Scale::new(100.0, 20.0)),
            //Transform::new(Position::new(200.0, 2000.0), 0.0, Scale::new(100.0, 100.0)),
        ];

        let sprite_instances: Vec<SpriteInstance> = transforms
            .iter()
            .map(|t| SpriteInstance::new(t, region))
            .collect();

        self.assets_manager
            .instances
            .write_slice(&self.queue, &sprite_instances);

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        // 3. Render Pass
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, atlas_bind_group, &[]);
            rpass.set_bind_group(1, &self.assets_manager.instances.bind_group, &[]);
            rpass.set_bind_group(2, &self.assets_manager.camera.bind_group, &[]);
            rpass.draw(0..6, 0..sprite_instances.len() as u32);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    pub fn render(&mut self, interpolation: f32) -> Result<(), SurfaceError> {
        dbg!(interpolation);
        self.draw(interpolation);
        Ok(())
    }

    fn create_surface_config(
        size: PhysicalSize<u32>,
        caps: &SurfaceCapabilities,
    ) -> SurfaceConfiguration {
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        }
    }

    async fn select_adapter(instance: &Instance, surface: &Surface<'_>) -> Adapter {
        instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapter found")
    }
}
