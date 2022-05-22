mod app;
mod camera;
mod frame_info;
mod game_object;
mod input;
mod keyboard_movement_controller;
mod network;
mod systems;
mod vulkan;
mod window;

pub use frame_info::*;
pub use input::*;
pub use keyboard_movement_controller::*;

use crate::app::App;

fn main() {
    simple_logger::SimpleLogger::new()
        .without_timestamps()
        .init()
        .unwrap();

    let event_loop = winit::event_loop::EventLoop::new();

    let app = App::new(&event_loop).unwrap();

    // for testing purposes
    let world = common::world::World::new(10, 10);

    App::run(app, event_loop).unwrap();
}
