mod device;
mod instance;
mod shader;

pub use device::*;
pub use instance::*;
pub use shader::*;

#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("")]
    VulkanError(#[from] ash::vk::Result),
    #[error("")]
    LoadingError(#[from] ash::LoadingError),
}
