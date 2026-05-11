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
        if self.game_renderer.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(WindowAttributes::default())
                    .unwrap(),
            );
            // Use block_on only for initialization
            let mut game_renderer =
                pollster::block_on(GameRenderer::new(window, "./assets/shader.wgsl"));

            let _ = game_renderer.assets_manager.load("./assets/test.png");
            self.game_renderer = Some(game_renderer);
        }
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

                // Step the logic as many times as needed to catch up
                while self.accumulator >= self.target_tps {
                    self.game_logic.update();
                    self.accumulator -= self.target_tps;
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
