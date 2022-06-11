use std::rc::Rc;

use crate::graphics::RenderError;

use super::Device;

pub struct ShaderModule {
    device: Rc<Device>,
    module: ash::vk::ShaderModule,
}

impl ShaderModule {
    pub unsafe fn new(
        device: Rc<Device>,
        file: rust_embed::EmbeddedFile,
    ) -> anyhow::Result<Rc<Self>, RenderError> {
        let code = ash::util::read_spv(&mut std::io::Cursor::new(file.data.as_ref()))
            .map_err(|e| log::error!("Unable to read file: {}", e))
            .unwrap();

        let create_info = ash::vk::ShaderModuleCreateInfo::builder().code(&code);

        let module = device
            .logical_device
            .create_shader_module(&create_info, None)?;

        Ok(Rc::new(Self { device, module }))
    }

    #[inline]
    pub fn inner(&self) -> ash::vk::ShaderModule {
        self.module
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        log::debug!("Dropping vulkan shader module");

        unsafe {
            self.device
                .logical_device
                .destroy_shader_module(self.module, None);
        }
    }
}
