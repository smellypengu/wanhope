use std::rc::Rc;

use super::{Buffer, Device, ImageView, RenderError};

pub struct Image {
    device: Rc<Device>,
    image: ash::vk::Image,
    image_memory: ash::vk::DeviceMemory,
    image_view: Rc<ImageView>,
    pub image_sampler: ash::vk::Sampler,
    pub image_info: ash::vk::DescriptorImageInfo,
    pub format: ash::vk::Format,
}

impl Image {
    pub fn from_raw(
        device: Rc<Device>,
        data: Vec<u8>,
        width: u32,
        height: u32,
        // TODO: figure out a better way to handle this parameter for egui
        image_sampler: ash::vk::Sampler,
    ) -> anyhow::Result<Rc<Self>, RenderError> {
        let size = (width * height * 4) as usize;

        let mut staging_buffer = unsafe {
            Buffer::new(
                device.clone(),
                size,
                ash::vk::BufferUsageFlags::TRANSFER_SRC,
                ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                    | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
            )?
        };

        unsafe {
            staging_buffer.map(0)?;
            staging_buffer.write_to_buffer(&data);
            staging_buffer.unmap();
        }

        let extent = ash::vk::Extent3D {
            width,
            height,
            depth: 1,
        };

        let image_info = ash::vk::ImageCreateInfo::builder()
            .image_type(ash::vk::ImageType::TYPE_2D)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .format(ash::vk::Format::R8G8B8A8_SRGB)
            .usage(ash::vk::ImageUsageFlags::TRANSFER_DST | ash::vk::ImageUsageFlags::SAMPLED)
            .samples(ash::vk::SampleCountFlags::TYPE_1);

        let (image, image_memory) = unsafe {
            device
                .create_image_with_info(&image_info, ash::vk::MemoryPropertyFlags::DEVICE_LOCAL)?
        };

        unsafe {
            Self::transition_image_layout(
                &device,
                image,
                ash::vk::ImageLayout::UNDEFINED,
                ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                1,
            )?;

            device.copy_buffer_to_image(staging_buffer.inner(), image, width, height, 1)?;

            Self::transition_image_layout(
                &device,
                image,
                ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                1,
            )?;
        }

        let image_view = ImageView::new(
            device.clone(),
            image,
            ash::vk::Format::R8G8B8A8_SRGB,
            ash::vk::ImageAspectFlags::COLOR,
        )?;

        let image_info = ash::vk::DescriptorImageInfo {
            sampler: image_sampler,
            image_view: image_view.inner(),
            image_layout: ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };

        Ok(Rc::new(Self {
            device,
            image,
            image_memory,
            image_view,
            image_sampler,
            image_info,
            format: ash::vk::Format::UNDEFINED,
        }))
    }

    pub fn from_file(
        device: Rc<Device>,
        file: rust_embed::EmbeddedFile,
    ) -> anyhow::Result<Rc<Self>, RenderError> {
        let image = image::load_from_memory(file.data.as_ref()).map(|img| img.to_rgba8())?;
        let (width, height) = image.dimensions();

        let image_sampler = unsafe { Self::create_texture_sampler(&device)? };

        Self::from_raw(device, image.into_raw(), width, height, image_sampler)
    }

    unsafe fn transition_image_layout(
        device: &Rc<Device>,
        image: ash::vk::Image,
        old_layout: ash::vk::ImageLayout,
        new_layout: ash::vk::ImageLayout,
        mip_levels: u32,
    ) -> anyhow::Result<(), RenderError> {
        let command_buffer = device.begin_single_time_commands()?;

        let src_access_mask;
        let dst_access_mask;
        let src_stage;
        let dst_stage;

        if old_layout == ash::vk::ImageLayout::UNDEFINED
            && new_layout == ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL
        {
            src_access_mask = ash::vk::AccessFlags::empty();
            dst_access_mask = ash::vk::AccessFlags::TRANSFER_WRITE;
            src_stage = ash::vk::PipelineStageFlags::TOP_OF_PIPE;
            dst_stage = ash::vk::PipelineStageFlags::TRANSFER;
        } else if old_layout == ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL
            && new_layout == ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        {
            src_access_mask = ash::vk::AccessFlags::TRANSFER_WRITE;
            dst_access_mask = ash::vk::AccessFlags::SHADER_READ;
            src_stage = ash::vk::PipelineStageFlags::TRANSFER;
            dst_stage = ash::vk::PipelineStageFlags::FRAGMENT_SHADER;
        } else if old_layout == ash::vk::ImageLayout::UNDEFINED
            && new_layout == ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        {
            src_access_mask = ash::vk::AccessFlags::empty();
            dst_access_mask = ash::vk::AccessFlags::COLOR_ATTACHMENT_READ
                | ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
            src_stage = ash::vk::PipelineStageFlags::TOP_OF_PIPE;
            dst_stage = ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
        } else {
            panic!("Unsupported layout transition!");
        }

        let image_barriers = [ash::vk::ImageMemoryBarrier {
            src_access_mask,
            dst_access_mask,
            old_layout,
            new_layout,
            src_queue_family_index: ash::vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: ash::vk::QUEUE_FAMILY_IGNORED,
            image,
            subresource_range: ash::vk::ImageSubresourceRange {
                aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        }];

        device.logical_device.cmd_pipeline_barrier(
            command_buffer,
            src_stage,
            dst_stage,
            ash::vk::DependencyFlags::empty(),
            &[],
            &[],
            &image_barriers,
        );

        device.end_single_time_commands(command_buffer)
    }

    unsafe fn create_texture_sampler(
        device: &Rc<Device>,
    ) -> anyhow::Result<ash::vk::Sampler, RenderError> {
        let sampler_create_info = ash::vk::SamplerCreateInfo {
            mag_filter: ash::vk::Filter::LINEAR,
            min_filter: ash::vk::Filter::LINEAR,
            mipmap_mode: ash::vk::SamplerMipmapMode::LINEAR,
            address_mode_u: ash::vk::SamplerAddressMode::REPEAT,
            address_mode_v: ash::vk::SamplerAddressMode::REPEAT,
            address_mode_w: ash::vk::SamplerAddressMode::REPEAT,
            mip_lod_bias: 0.0,
            anisotropy_enable: ash::vk::FALSE,
            max_anisotropy: 16.0,
            compare_enable: ash::vk::FALSE,
            compare_op: ash::vk::CompareOp::ALWAYS,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: ash::vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: ash::vk::FALSE,
            ..Default::default()
        };

        let sampler = device
            .logical_device
            .create_sampler(&sampler_create_info, None)?;

        Ok(sampler)
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        log::debug!("Dropping vulkan image");

        unsafe {
            self.device.logical_device.destroy_image(self.image, None);

            self.device
                .logical_device
                .free_memory(self.image_memory, None);

            self.device
                .logical_device
                .destroy_sampler(self.image_sampler, None);
        }
    }
}
