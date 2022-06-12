use std::rc::Rc;

use image::GenericImage;

use super::{
    vulkan::{Device, Image},
    RenderError,
};

pub struct TileAtlas {
    image_buffer: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    textures: Vec<image::DynamicImage>,

    pub size: u32,
    pub tile_size: u32,
}

impl TileAtlas {
    pub fn new(size: u32, tile_size: u32) -> anyhow::Result<Self, RenderError> {
        let image_buffer = image::ImageBuffer::new(size * tile_size, size * tile_size);

        Ok(Self {
            image_buffer,
            textures: Vec::new(),

            size,
            tile_size,
        })
    }

    pub fn add_texture(
        &mut self,
        asset: rust_embed::EmbeddedFile,
    ) -> anyhow::Result<(), RenderError> {
        let image = image::load_from_memory(asset.data.as_ref())?;

        self.textures.push(image);

        Ok(())
    }

    pub fn build(&mut self, device: Rc<Device>) -> anyhow::Result<Rc<Image>, RenderError> {
        let mut x = 0;
        let mut y = 0;

        for texture in &self.textures {
            if x == self.size * self.tile_size {
                x = 0;
                y += self.tile_size;
            }

            if y == self.size * self.tile_size {
                // out of bounds
                return Err(RenderError::TextureAtlasError);
            }

            self.image_buffer.copy_from(texture, x, y)?;

            x += self.tile_size;
        }

        let sampler_create_info = ash::vk::SamplerCreateInfo {
            mag_filter: ash::vk::Filter::NEAREST,
            min_filter: ash::vk::Filter::NEAREST,
            mipmap_mode: ash::vk::SamplerMipmapMode::NEAREST,
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

        Image::from_raw(
            device,
            self.image_buffer.clone().into_raw(),
            self.image_buffer.width(),
            self.image_buffer.height(),
            sampler,
        )
    }
}
