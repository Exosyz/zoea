use crate::assets_manager::AssetManager;
use crate::backend::utils::build_rendering_pipeline;
use crate::instance::camera::CameraInstance;
use crate::instance::sprite::SpriteInstance;
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::{
    Adapter, Color, CommandEncoderDescriptor, CurrentSurfaceTexture, Device, DeviceDescriptor,
    Instance, LoadOp, Operations, PresentMode, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RequestAdapterOptions, StoreOp, Surface,
    SurfaceCapabilities, SurfaceConfiguration, SurfaceTarget, TextureUsages, TextureViewDescriptor,
};
use zoea_core::rendering::assets::{AssetId, DrawInstance};
use zoea_core::size::Size;

pub struct GameRenderer<'window> {
    surface: Surface<'window>,
    config: SurfaceConfiguration,
    device: Arc<Device>,
    queue: Arc<Queue>,
    render_pipeline: RenderPipeline,

    pub assets_manager: AssetManager,
    pub camera: CameraInstance,
}

impl<'window> GameRenderer<'window> {
    pub async fn new(target: impl Into<SurfaceTarget<'window>>, window_size: Size) -> Self {
        let instance = Instance::default();
        let surface = instance.create_surface(target).expect("Surface failed");

        let adapter = Self::select_adapter(&instance, &surface).await;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default())
            .await
            .expect("Device failed");

        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let caps = surface.get_capabilities(&adapter);
        let config = Self::create_surface_config(window_size, &caps);

        surface.configure(&device, &config);

        let assets_manager = AssetManager::new(device.clone(), queue.clone());
        let render_pipeline = build_rendering_pipeline(device.clone(), &config, &assets_manager);

        dbg!(window_size);
        let camera = CameraInstance::from_size(window_size);

        Self {
            surface,
            config,
            device,
            queue,
            render_pipeline,

            assets_manager,
            camera,
        }
    }

    pub fn resize(&mut self, new_size: Size) {
        let new_width = new_size.width() as u32;
        let new_height = new_size.height() as u32;
        if new_width > 0 && new_height > 0 {
            self.config.width = new_width;
            self.config.height = new_height;

            self.surface.configure(&self.device, &self.config);
            self.camera = CameraInstance::from_size(new_size);
        }
    }

    pub fn draw(&mut self, sprites: HashMap<AssetId, Vec<DrawInstance>>) {
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

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

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
            rpass.set_bind_group(2, &self.assets_manager.camera.bind_group, &[]);

            for (asset_id, instances) in sprites {
                let Some(asset_bind_group) = self.assets_manager.get(asset_id) else {
                    continue;
                };

                let sprite_instances: Vec<SpriteInstance> = instances
                    .iter()
                    .map(SpriteInstance::from)
                    .collect();

                self.assets_manager.instances.write_slice(
                    &self.queue,
                    &sprite_instances,
                );

                // Update bind groups and issue draw command
                rpass.set_bind_group(0, asset_bind_group, &[]);
                rpass.set_bind_group(1, &self.assets_manager.instances.bind_group, &[]);
                rpass.draw(0..6, 0..instances.len() as u32);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    fn create_surface_config(size: Size, caps: &SurfaceCapabilities) -> SurfaceConfiguration {
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width().max(1.0) as u32,
            height: size.height().max(1.0) as u32,
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
