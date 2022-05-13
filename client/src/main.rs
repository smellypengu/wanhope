mod vulkan;
mod app;
mod camera;
mod frame_info;
mod game_object;
mod input;
mod keyboard_movement_controller;
mod simple_render_system;
mod window;

pub use frame_info::*;
pub use input::*;
pub use keyboard_movement_controller::*;
pub use simple_render_system::*;

use crate::app::App;

fn main() {
    simple_logger::SimpleLogger::new().without_timestamps().init().unwrap();
    
    let event_loop = winit::event_loop::EventLoop::new();
    
    let app = App::new(&event_loop);

    App::run(app, event_loop);
}
