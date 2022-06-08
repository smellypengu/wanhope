mod app;
mod egui;
mod game_object;
mod graphics;
mod input;
mod keyboard_movement_controller;
mod network;

pub use input::*;
pub use keyboard_movement_controller::*;

use crate::app::App;

#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
pub struct Asset;

fn main() {
    simple_logger::SimpleLogger::new()
        .without_timestamps()
        .init()
        .unwrap();

    let event_loop = winit::event_loop::EventLoop::new();

    let app = App::new(&event_loop).unwrap();

    App::run(app, event_loop).unwrap();
}
