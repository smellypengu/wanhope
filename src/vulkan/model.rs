use std::rc::Rc;

use super::{RenderError, Device, Buffer};

#[derive(Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: glam::Vec2,
    pub color: glam::Vec3,
}

impl Vertex {
    pub fn binding_descriptions() -> Vec<ash::vk::VertexInputBindingDescription> {
        let vertex_size = std::mem::size_of::<Vertex>() as u32;

        vec![
            ash::vk::VertexInputBindingDescription {
                binding: 0,
                stride: vertex_size,
                input_rate: ash::vk::VertexInputRate::VERTEX,
            },
        ]
    }

    pub fn attribute_descriptions() -> Vec<ash::vk::VertexInputAttributeDescription> {
        vec![
            ash::vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: ash::vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            ash::vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: ash::vk::Format::R32G32B32_SFLOAT,
                offset: std::mem::size_of::<glam::Vec2>() as u32,
            },
        ]
    }
}

pub struct Model {
    vertex_buffer: Buffer<Vertex>,
    vertex_count: u32,
    indices: Option<(Buffer<u32>, u32)>,
}

impl Model {
    pub fn new(
        device: Rc<Device>,
        vertices: &Vec<Vertex>,
        indices: Option<&Vec<u32>>,
    ) -> anyhow::Result<Rc<Self>, RenderError> {
        let (vertex_buffer, vertex_count) =
            Self::create_vertex_buffers(&device, vertices)?;

        match indices {
            Some(indices) => {
                let indices = Self::create_index_buffers(&device, indices)?;

                return Ok(Rc::new(Self {
                    vertex_buffer,
                    vertex_count,
                    indices: Some(indices),
                }));
            },
            None => { }
        }

        Ok(Rc::new(Self {
            vertex_buffer,
            vertex_count,
            indices: None,
        }))
    }

    pub unsafe fn draw(&self, logical_device: &ash::Device, command_buffer: ash::vk::CommandBuffer) {
        match &self.indices {
            Some((_index_buffer, index_count)) => {
                logical_device.cmd_draw_indexed(
                    command_buffer,
                    *index_count,
                    1,
                    0,
                    0,
                    0,
                );
            },
            None => {
                logical_device.cmd_draw(
                    command_buffer, 
                    self.vertex_count,
                    1,
                    0,
                    0,
                );
            }
        }
    }

    pub unsafe fn bind(&self, command_buffer: ash::vk::CommandBuffer) {
        self.vertex_buffer.bind_vertex(command_buffer);

        match &self.indices {
            Some((index_buffer, _index_count)) => {
                index_buffer.bind_index(command_buffer, ash::vk::IndexType::UINT32);
            },
            None => { }
        }
    }

    fn create_vertex_buffers(
        device: &Rc<Device>,
        vertices: &Vec<Vertex>,
    ) -> anyhow::Result<(Buffer<Vertex>, u32), RenderError> {
        let vertex_count = vertices.len();

        assert!(
            vertex_count >= 3,
            "Vertex count must be at least 3",
        );

        let buffer_size: ash::vk::DeviceSize = (std::mem::size_of::<Vertex>() * vertex_count) as u64;

        let mut staging_buffer = Buffer::new(
            device.clone(),
            vertex_count,
            ash::vk::BufferUsageFlags::TRANSFER_SRC,
            ash::vk::MemoryPropertyFlags::HOST_VISIBLE | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            staging_buffer.map(0)?;
            staging_buffer.write_to_buffer(vertices);
        }

        let vertex_buffer = Buffer::new(
            device.clone(),
            vertex_count,
            ash::vk::BufferUsageFlags::VERTEX_BUFFER | ash::vk::BufferUsageFlags::TRANSFER_DST,
            ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        device.copy_buffer(staging_buffer.inner(), vertex_buffer.inner(), buffer_size)?;

        Ok((vertex_buffer, vertex_count as u32))
    }

    fn create_index_buffers(
        device: &Rc<Device>,
        indices: &Vec<u32>,
    ) -> anyhow::Result<(Buffer<u32>, u32), RenderError> {
        let index_count = indices.len();

        let buffer_size: ash::vk::DeviceSize = (std::mem::size_of::<u32>() * index_count) as u64;

        let mut staging_buffer = Buffer::new(
            device.clone(),
            index_count,
            ash::vk::BufferUsageFlags::TRANSFER_SRC,
            ash::vk::MemoryPropertyFlags::HOST_VISIBLE | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            staging_buffer.map(0)?;
            staging_buffer.write_to_buffer(indices);
        }

        let index_buffer = Buffer::new(
            device.clone(),
            index_count,
            ash::vk::BufferUsageFlags::INDEX_BUFFER | ash::vk::BufferUsageFlags::TRANSFER_DST,
            ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        device.copy_buffer(staging_buffer.inner(), index_buffer.inner(), buffer_size)?;

        Ok((index_buffer, index_count as u32))
    }
}
