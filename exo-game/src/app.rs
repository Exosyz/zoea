use crate::game_logic::GameLogic;
use crate::game_renderer::GameRenderer;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowAttributes;

pub struct App<'window> {
    game_renderer: Option<GameRenderer<'window>>,
    game_logic: GameLogic,
    last_tick_time: Instant,
    accumulator: Duration,
    target_tps: Duration,
}

impl Default for App<'_> {
    fn default() -> Self {
        Self {
            game_renderer: None,
            game_logic: Default::default(),
            last_tick_time: Instant::now(),
            accumulator: Default::default(),
            target_tps: Duration::from_secs_f64(1.0 / 60.0), // 60 TPS
        }
    }
}

impl<'window> ApplicationHandler for App<'window> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.game_renderer.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        // Block_on is acceptable here as it's a one-time startup cost
        let mut renderer = pollster::block_on(GameRenderer::new(window));

        // Note: Production engines usually load a 'manifest' file rather than hardcoding here
        renderer
            .assets_manager
            .load("./assets/terrain_tiles_v2.png", 320, 512, |atlas| {
                atlas
                    .add_sprite(0, 0, 32, 32)
                    .add_sprite(0, 32, 32, 32)
                    .add_sprite(32, 32, 32, 32);
            })
            .expect("Failed to load core atlas");

        self.game_renderer = Some(renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let game_renderer = match self.game_renderer.as_mut() {
            Some(s) => s,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                game_renderer.resize(size);
                game_renderer.window.request_redraw();
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
                    self.game_logic.update();
                    self.accumulator -= self.target_tps;
                    iterations += 1;
                }

                // Calculate how far we are into the NEXT frame
                let interpolation = self.accumulator.as_secs_f32() / self.target_tps.as_secs_f32();

                game_renderer.render(interpolation).unwrap();
                // Constant loop for smooth rendering on WSLg
                game_renderer.window.request_redraw();
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
                dbg!(device_id, is_synthetic, device_id);
            }
            _ => (),
        }
    }
}
