mod camera;
mod frame_info;
mod plane;
mod ray;
pub mod systems;
mod texture_atlas;
pub mod vulkan;
mod window;

pub use camera::*;
pub use frame_info::*;
pub use plane::*;
pub use ray::*;
pub use texture_atlas::*;
pub use window::*;

#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("")]
    TextureAtlasError,
    #[error("")]
    ImageError(#[from] ::image::ImageError),
    #[error("")]
    VulkanError(#[from] ash::vk::Result),
    #[error("")]
    LoadingError(#[from] ash::LoadingError),
    #[error("Swapchain image or depth format has changed")]
    CompareSwapFormatsError,
}
