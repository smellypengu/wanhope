pub mod descriptor_set;
mod buffer;
mod device;
mod image_view;
mod instance;
mod model;
mod pipeline;
mod renderer;
mod shader;
mod swapchain;

pub use buffer::*;
pub use device::*;
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

#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("")]
    VulkanError(#[from] ash::vk::Result),
    #[error("")]
    LoadingError(#[from] ash::LoadingError),
    #[error("Swapchain image or depth format has changed")]
    CompareSwapFormatsError,
}
