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
#[folder = "assets/models"]
pub struct ModelAsset;

#[derive(rust_embed::RustEmbed)]
#[folder = "assets/shaders"]
pub struct ShaderAsset;

#[derive(rust_embed::RustEmbed)]
#[folder = "assets/textures"]
pub struct TextureAsset;

fn main() {
    simple_logger::SimpleLogger::new()
        .without_timestamps()
        .init()
        .unwrap();

    let event_loop = winit::event_loop::EventLoop::new();

    let app = App::new(&event_loop).unwrap();

    App::run(app, event_loop).unwrap();
}
