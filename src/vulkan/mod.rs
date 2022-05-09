mod device;
mod image_view;
mod instance;
mod pipeline;
mod shader;
mod swapchain;

pub use device::*;
pub use image_view::*;
pub use instance::*;
pub use pipeline::*;
pub use shader::*;
pub use swapchain::*;

#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("")]
    VulkanError(#[from] ash::vk::Result),
    #[error("")]
    LoadingError(#[from] ash::LoadingError),
    #[error("Swapchain image or depth format has changed")]
    CompareSwapFormatsError,
}
