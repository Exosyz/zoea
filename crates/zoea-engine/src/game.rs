use crate::scene::Scene;
use crate::scene_manager::{SceneId, SceneManager};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes};
use zoea_assets::atlas::Atlas;
use zoea_core::rendering::assets::AssetId;
use zoea_core::size::Size;
use zoea_rendering::renderer::GameRenderer;

pub struct GameEngine<'window> {
    window: Option<Arc<Window>>,
    last_tick_time: Instant,
    accumulator: Duration,
    target_tps: Duration,

    game_renderer: Option<GameRenderer<'window>>,
    atlases: HashMap<&'window str, Atlas<'window>>,
    scene_manager: SceneManager,
}

impl<'window> GameEngine<'window> {
    pub fn start(&mut self) {
        let event_loop = EventLoop::new().unwrap();
        // Poll flow is better for games, Wait is better for power saving.
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self).unwrap();
    }

    fn request_redraw(&mut self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub fn add_atlas(&mut self, mut atlas: Atlas<'window>) -> Option<AssetId> {
        let id = Self::reload_atlas_internal(self.game_renderer.as_mut(), &mut atlas);
        self.atlases.insert(atlas.source, atlas);
        id
    }

    fn reload_atlases(&mut self) {
        let mut game_renderer = self.game_renderer.as_mut();

        for atlas in self.atlases.values_mut() {
            Self::reload_atlas_internal(game_renderer.as_deref_mut(), atlas);
        }
    }

    fn reload_atlas_internal(
        game_renderer: Option<&mut GameRenderer<'window>>,
        atlas: &mut Atlas<'window>,
    ) -> Option<AssetId> {
        let Some(game_renderer) = game_renderer else { return None };

        let id = game_renderer
            .assets_manager
            .load(atlas.source)
            .expect(&format!("Fail to load the atlas at {}", atlas.source));

        atlas.set_id(id);
        Some(id)
    }

    pub fn add_scene(&mut self, scene: Scene) -> SceneId {
        self.scene_manager.add_scene(scene)
    }

    pub fn select_scene(&mut self, scene_id: SceneId) {
        self.scene_manager.select_scene(scene_id)
    }

    pub fn tick(&mut self) {
        let Some(scene) = self.scene_manager.current_scene() else { return };

        scene.tick();
    }

    pub fn render(&mut self, interpolation: f32) {
        let Some(scene) = self.scene_manager.current_scene() else { return };
        let Some(game_renderer) = self.game_renderer.as_mut()  else { return };

        scene.render(game_renderer, interpolation);
    }
}

impl Default for GameEngine<'_> {
    fn default() -> Self {
        Self {
            window: None,
            game_renderer: None,
            last_tick_time: Instant::now(),
            accumulator: Default::default(),
            target_tps: Duration::from_secs_f64(1.0 / 60.0), // 60 TPS

            atlases: Default::default(),
            scene_manager: Default::default(),
        }
    }
}

impl<'window> ApplicationHandler for GameEngine<'window> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.game_renderer.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        self.window = Some(window.clone());

        let window_inner_size = window.inner_size();
        // Block_on is acceptable here as it's a one-time startup cost
        let mut renderer = pollster::block_on(GameRenderer::new(
            window,
            Size::new(
                window_inner_size.width as f32,
                window_inner_size.height as f32,
            ),
        ));

        for (source, atlas) in self.atlases.iter_mut() {
            let asset_id = renderer
                .assets_manager
                .load(source)
                .expect(format!("Failed to load atlas : {source}").as_str());

            atlas.register_atlas(asset_id);
        }

        self.game_renderer = Some(renderer);

        self.reload_atlases()
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                let wrapped_size = Size::new(size.width as f32, size.height as f32);
                let game_renderer = match self.game_renderer.as_mut() {
                    Some(s) => s,
                    None => return,
                };
                game_renderer.resize(wrapped_size);
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let frame_time = now - self.last_tick_time;
                self.last_tick_time = now;

                // Add elapsed time to our "bucket"
                self.accumulator += frame_time;
                let mut iterations = 0;
                // Step the logic as many times as needed to catch up
                while self.accumulator >= self.target_tps && iterations < 10 {
                    self.tick();
                    self.accumulator -= self.target_tps;
                    iterations += 1;
                }

                // Calculate how far we are into the NEXT frame
                let interpolation = self.accumulator.as_secs_f32() / self.target_tps.as_secs_f32();

                self.render(interpolation);
                // Constant loop for smooth rendering on WSLg
                self.request_redraw();
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                dbg!(device_id, state, button);
            }
            WindowEvent::KeyboardInput {
                event,
                device_id,
                is_synthetic,
            } => {
                dbg!(event, is_synthetic, device_id);
            }
            _ => (),
        }
    }
}
