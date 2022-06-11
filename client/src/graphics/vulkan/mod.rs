mod buffer;
pub mod descriptor_set;
mod device;
mod egui_integration;
mod image;
mod image_view;
mod instance;
mod model;
mod pipeline;
mod renderer;
mod shader;
mod swapchain;

pub use self::image::*;
pub use buffer::*;
pub use device::*;
pub use egui_integration::*;
pub use image_view::*;
pub use instance::*;
pub use model::*;
pub use pipeline::*;
pub use renderer::*;
pub use shader::*;
pub use swapchain::*;

#[repr(align(16))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Align16<T>(pub T);
