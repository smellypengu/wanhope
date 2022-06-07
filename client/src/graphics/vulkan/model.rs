use std::{mem::size_of, rc::Rc};

use super::{Buffer, Device, RenderError};

#[derive(Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: glam::Vec3,
    pub color: glam::Vec3,
    pub normal: glam::Vec3,
    pub uv: glam::Vec2,
}

impl Vertex {
    pub fn binding_descriptions() -> Vec<ash::vk::VertexInputBindingDescription> {
        let vertex_size = std::mem::size_of::<Vertex>() as u32;

        vec![ash::vk::VertexInputBindingDescription {
            binding: 0,
            stride: vertex_size,
            input_rate: ash::vk::VertexInputRate::VERTEX,
        }]
    }

    pub fn attribute_descriptions() -> Vec<ash::vk::VertexInputAttributeDescription> {
        vec![
            ash::vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: ash::vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            ash::vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: ash::vk::Format::R32G32B32_SFLOAT,
                offset: size_of::<glam::Vec3>() as u32,
            },
            ash::vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: ash::vk::Format::R32G32B32_SFLOAT,
                offset: (size_of::<glam::Vec3>() + size_of::<glam::Vec3>()) as u32,
            },
            ash::vk::VertexInputAttributeDescription {
                location: 3,
                binding: 0,
                format: ash::vk::Format::R32G32B32_SFLOAT,
                offset: (size_of::<glam::Vec3>()
                    + size_of::<glam::Vec3>()
                    + size_of::<glam::Vec3>()) as u32,
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
            unsafe { Self::create_vertex_buffers(&device, vertices)? };

        match indices {
            Some(indices) => {
                let indices = unsafe { Self::create_index_buffers(&device, indices)? };

                return Ok(Rc::new(Self {
                    vertex_buffer,
                    vertex_count,
                    indices: Some(indices),
                }));
            }
            None => {}
        }

        Ok(Rc::new(Self {
            vertex_buffer,
            vertex_count,
            indices: None,
        }))
    }

    pub fn from_file(device: Rc<Device>, file_path: &str) -> anyhow::Result<Rc<Self>, RenderError> {
        let (models, _) = tobj::load_obj(file_path, &tobj::GPU_LOAD_OPTIONS).unwrap();

        let mesh = &models[0].mesh;

        let positions = mesh.positions.as_slice();

        let colors = match mesh.vertex_color.as_slice() {
            [] => vec![1_f32; positions.len()],
            v => v.to_vec(),
        };

        let normals = mesh.normals.as_slice();
        let uvs = mesh.texcoords.as_slice();

        let vertex_count = mesh.positions.len() / 3;

        let mut vertices = Vec::with_capacity(vertex_count);
        for i in 0..vertex_count {
            let x = positions[3 * i + 0];
            let y = positions[3 * i + 1];
            let z = positions[3 * i + 2];

            let color_x = colors[3 * i + 0];
            let color_y = colors[3 * i + 1];
            let color_z = colors[3 * i + 2];

            let normal_x = normals[3 * i + 0];
            let normal_y = normals[3 * i + 1];
            let normal_z = normals[3 * i + 2];

            let u = uvs[2 * i + 0];
            let v = uvs[2 * i + 1];

            let vertex = Vertex {
                position: glam::vec3(x, y, z),
                color: glam::vec3(color_x, color_y, color_z),
                normal: glam::vec3(normal_x, normal_y, normal_z),
                uv: glam::vec2(u, v),
            };

            vertices.push(vertex);
        }

        Ok(Model::new(device, &vertices, Some(&mesh.indices.clone()))?)
    }

    pub unsafe fn draw(
        &self,
        logical_device: &ash::Device,
        command_buffer: ash::vk::CommandBuffer,
    ) {
        match &self.indices {
            Some((_index_buffer, index_count)) => {
                logical_device.cmd_draw_indexed(command_buffer, *index_count, 1, 0, 0, 0);
            }
            None => {
                logical_device.cmd_draw(command_buffer, self.vertex_count, 1, 0, 0);
            }
        }
    }

    pub unsafe fn bind(&self, command_buffer: ash::vk::CommandBuffer) {
        self.vertex_buffer.bind_vertex(command_buffer);

        match &self.indices {
            Some((index_buffer, _index_count)) => {
                index_buffer.bind_index(command_buffer, ash::vk::IndexType::UINT32);
            }
            None => {}
        }
    }

    unsafe fn create_vertex_buffers(
        device: &Rc<Device>,
        vertices: &Vec<Vertex>,
    ) -> anyhow::Result<(Buffer<Vertex>, u32), RenderError> {
        let vertex_count = vertices.len();

        assert!(vertex_count >= 3, "Vertex count must be at least 3",);

        let buffer_size: ash::vk::DeviceSize =
            (std::mem::size_of::<Vertex>() * vertex_count) as u64;

        let mut staging_buffer = Buffer::new(
            device.clone(),
            vertex_count,
            ash::vk::BufferUsageFlags::TRANSFER_SRC,
            ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        staging_buffer.map(0)?;
        staging_buffer.write_to_buffer(vertices);

        let vertex_buffer = Buffer::new(
            device.clone(),
            vertex_count,
            ash::vk::BufferUsageFlags::VERTEX_BUFFER | ash::vk::BufferUsageFlags::TRANSFER_DST,
            ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        device.copy_buffer(staging_buffer.inner(), vertex_buffer.inner(), buffer_size)?;

        Ok((vertex_buffer, vertex_count as u32))
    }

    unsafe fn create_index_buffers(
        device: &Rc<Device>,
        indices: &Vec<u32>,
    ) -> anyhow::Result<(Buffer<u32>, u32), RenderError> {
        let index_count = indices.len();

        let buffer_size: ash::vk::DeviceSize = (std::mem::size_of::<u32>() * index_count) as u64;

        let mut staging_buffer = Buffer::new(
            device.clone(),
            index_count,
            ash::vk::BufferUsageFlags::TRANSFER_SRC,
            ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        staging_buffer.map(0)?;
        staging_buffer.write_to_buffer(indices);

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
