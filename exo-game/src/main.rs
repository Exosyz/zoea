mod app;
mod assets_manager;
mod game_logic;
mod game_renderer;
mod math;
mod samplers;

use crate::app::App;
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    // Poll flow is better for games, Wait is better for power saving.
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
