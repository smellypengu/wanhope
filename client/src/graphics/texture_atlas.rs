use std::rc::Rc;

use image::GenericImage;

use super::{
    vulkan::{Device, Image},
    RenderError,
};

pub struct TextureAtlas {
    image_buffer: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,

    pub size: u32,
    pub tile_size: u32,
}

impl TextureAtlas {
    pub fn new(
        size: u32,
        tile_size: u32,
        textures: Vec<rust_embed::EmbeddedFile>,
    ) -> anyhow::Result<Self, RenderError> {
        let mut image_buffer = image::ImageBuffer::new(size * tile_size, size * tile_size);

        let mut x = 0;
        let mut y = 0;

        for texture in textures {
            if x == size * tile_size {
                x = 0;
                y += tile_size;
            }

            if y == size * tile_size {
                // out of bounds
                return Err(RenderError::TextureAtlasError);
            }

            let image = image::load_from_memory(texture.data.as_ref())?;

            image_buffer.copy_from(&image, x, y)?;

            x += tile_size;
        }

        Ok(Self {
            image_buffer,
            size,
            tile_size,
        })
    }

    pub fn to_vulkan(&self, device: Rc<Device>) -> anyhow::Result<Rc<Image>, RenderError> {
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

        let sampler = unsafe {
            device
                .logical_device
                .create_sampler(&sampler_create_info, None)?
        };

        Ok(Image::from_raw(
            device,
            self.image_buffer.clone().into_raw(),
            self.image_buffer.width(),
            self.image_buffer.height(),
            sampler,
        )?)
    }
}
