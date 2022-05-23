use std::rc::Rc;

use super::{Device, RenderError};

pub struct ImageView {
    device: Rc<Device>,
    pub view: ash::vk::ImageView,
}

impl ImageView {
    pub fn new(
        device: Rc<Device>,
        image: ash::vk::Image,
        format: ash::vk::Format,
        aspect_mask: ash::vk::ImageAspectFlags,
    ) -> anyhow::Result<Rc<Self>, RenderError> {
        let view = unsafe {
            device.logical_device.create_image_view(
                &ash::vk::ImageViewCreateInfo::builder()
                    .image(image)
                    .format(format)
                    .view_type(ash::vk::ImageViewType::TYPE_2D)
                    .subresource_range(ash::vk::ImageSubresourceRange {
                        aspect_mask,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    }),
                None,
            )?
        };

        Ok(Rc::new(Self { device, view }))
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        log::debug!("Dropping vulkan image view");

        unsafe {
            self.device
                .logical_device
                .destroy_image_view(self.view, None);
        }
    }
}
