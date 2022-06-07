use std::rc::Rc;

use super::{Device, RenderError};

pub struct ShaderModule {
    device: Rc<Device>,
    module: ash::vk::ShaderModule,
}

impl ShaderModule {
    pub unsafe fn new<P: AsRef<std::path::Path>>(
        device: Rc<Device>,
        file_path: P,
    ) -> anyhow::Result<Rc<Self>, RenderError> {
        let code = Self::read_file(file_path);

        let create_info = ash::vk::ShaderModuleCreateInfo::builder().code(&code);

        let module = device
            .logical_device
            .create_shader_module(&create_info, None)?;

        Ok(Rc::new(Self { device, module }))
    }

    fn read_file<P: AsRef<std::path::Path>>(file_path: P) -> Vec<u32> {
        log::debug!(
            "Loading shader file: {}",
            file_path.as_ref().to_str().unwrap()
        );

        let mut file = std::fs::File::open(file_path)
            .map_err(|e| log::error!("Unable to open file: {}", e))
            .unwrap();

        ash::util::read_spv(&mut file)
            .map_err(|e| log::error!("Unable to read file: {}", e))
            .unwrap()
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
