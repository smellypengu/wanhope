use std::{ffi::c_void, rc::Rc};

use crate::graphics::{
    vulkan::{
        descriptor_set::{DescriptorPool, DescriptorSetLayout, DescriptorSetWriter},
        Buffer, Device, ImageView, Pipeline, RenderError, Swapchain,
    },
    Window,
};

pub struct EGuiIntegration {
    pub egui_ctx: egui::Context,
    pub egui_winit: egui_winit::State,

    physical_width: u32,
    physical_height: u32,
    scale_factor: f64,

    device: Rc<Device>,
    descriptor_pool: Rc<DescriptorPool>,
    descriptor_set_layouts: Vec<Rc<DescriptorSetLayout>>,
    pipeline_layout: ash::vk::PipelineLayout,
    pipeline: Rc<Pipeline>,
    sampler: ash::vk::Sampler,
    render_pass: ash::vk::RenderPass,
    framebuffer_color_image_views: Vec<Rc<ImageView>>,
    framebuffers: Vec<ash::vk::Framebuffer>,
    vertex_buffers: Vec<Buffer<egui::epaint::Vertex>>,
    index_buffers: Vec<Buffer<u32>>,
    font_image_staging_buffer: Buffer<u8>,
    font_image: (ash::vk::Image, ash::vk::DeviceMemory),
    font_image_view: Option<Rc<ImageView>>,
    font_image_size: [usize; 2],
    font_descriptor_sets: Vec<ash::vk::DescriptorSet>,

    user_texture_layout: Rc<DescriptorSetLayout>,
    user_textures: Vec<Option<ash::vk::DescriptorSet>>,

    shapes: Vec<egui::epaint::ClippedShape>,
    textures_delta: egui::TexturesDelta,
}

impl EGuiIntegration {
    pub fn new(
        window: &Window,
        device: Rc<Device>,
        swapchain: &Swapchain,
        surface_format: ash::vk::Format,
    ) -> anyhow::Result<Self, RenderError> {
        let egui_ctx = egui::Context::default();

        let descriptor_pool = unsafe {
            DescriptorPool::new(device.clone())
                .max_sets(1024)
                .pool_size(ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 1024)
                .build()?
        };

        let descriptor_set_layouts = {
            let mut sets = Vec::new();
            for _ in 0..swapchain.swapchain_images.len() {
                let layout = unsafe {
                    DescriptorSetLayout::new(device.clone())
                        .add_binding(
                            0,
                            ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                            ash::vk::ShaderStageFlags::FRAGMENT,
                            1,
                        )
                        .build()?
                };

                sets.push(layout)
            }

            sets
        };

        let render_pass = Self::create_render_pass(&device, surface_format)?;

        let pipeline_layout = unsafe {
            device.logical_device.create_pipeline_layout(
                &ash::vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(
                        &descriptor_set_layouts
                            .iter()
                            .map(|l| l.inner())
                            .collect::<Vec<_>>(),
                    ) // make simpler?
                    .push_constant_ranges(&[ash::vk::PushConstantRange::builder()
                        .stage_flags(ash::vk::ShaderStageFlags::VERTEX)
                        .offset(0)
                        .size(std::mem::size_of::<f32>() as u32 * 2)
                        .build()]),
                None,
            )?
        };

        let pipeline = Self::create_pipeline(
            device.clone(),
            &render_pass,
            &pipeline_layout,
            4 * std::mem::size_of::<f32>() as u32 + 4 * std::mem::size_of::<u8>() as u32,
        )?;

        let sampler = unsafe {
            device.logical_device.create_sampler(
                &ash::vk::SamplerCreateInfo::builder()
                    .address_mode_u(ash::vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_v(ash::vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_w(ash::vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .anisotropy_enable(false)
                    .min_filter(ash::vk::Filter::LINEAR)
                    .mag_filter(ash::vk::Filter::LINEAR)
                    .mipmap_mode(ash::vk::SamplerMipmapMode::LINEAR)
                    .min_lod(0.0)
                    .max_lod(ash::vk::LOD_CLAMP_NONE),
                None,
            )?
        };

        let (framebuffer_color_image_views, framebuffers) = Self::create_framebuffers(
            device.clone(),
            window,
            swapchain,
            render_pass,
            surface_format,
        );

        let mut vertex_buffers = Vec::new();
        let mut index_buffers = Vec::new();

        for _ in 0..framebuffers.len() {
            let mut vertex_buffer = Buffer::new(
                device.clone(),
                Self::vertex_buffer_size(),
                ash::vk::BufferUsageFlags::VERTEX_BUFFER,
                ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                    | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;

            let mut index_buffer = Buffer::new(
                device.clone(),
                Self::index_buffer_size(),
                ash::vk::BufferUsageFlags::INDEX_BUFFER,
                ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                    | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;

            unsafe {
                vertex_buffer.map(0)?;
                index_buffer.map(0)?;
            }

            vertex_buffers.push(vertex_buffer);
            index_buffers.push(index_buffer);
        }

        let user_texture_layout = unsafe {
            DescriptorSetLayout::new(device.clone())
                .add_binding(
                    0,
                    ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    ash::vk::ShaderStageFlags::FRAGMENT,
                    1,
                )
                .build()?
        };

        let font_image_staging_buffer = Buffer::new(
            device.clone(),
            1,
            ash::vk::BufferUsageFlags::TRANSFER_SRC,
            ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
        )?; // Would like to not do it like this but that's for future me to fix.

        let user_textures = Vec::new();

        Ok(Self {
            egui_ctx,
            egui_winit: egui_winit::State::new(
                device.properties.limits.max_image_dimension2_d as usize,
                &window.inner(),
            ),

            physical_width: window.inner().inner_size().width,
            physical_height: window.inner().inner_size().height,
            scale_factor: window.inner().scale_factor(),

            device,
            descriptor_pool,
            descriptor_set_layouts,
            pipeline_layout,
            pipeline,
            sampler,
            render_pass,
            framebuffer_color_image_views,
            framebuffers,
            vertex_buffers,
            index_buffers,
            font_image_staging_buffer,
            font_image: Default::default(),
            font_image_view: Default::default(),
            font_image_size: [0; 2],
            font_descriptor_sets: Vec::new(),

            user_texture_layout,
            user_textures,

            shapes: Vec::new(),
            textures_delta: Default::default(),
        })
    }

    fn vertex_buffer_size() -> usize {
        1024 * 1024 * 4 as usize
    }

    fn index_buffer_size() -> usize {
        1024 * 1024 * 2 as usize
    }

    pub fn begin_frame(&mut self, window: &Window) {
        self.egui_ctx
            .begin_frame(self.egui_winit.take_egui_input(&window.inner()));
    }

    pub fn on_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        match event {
            winit::event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = *scale_factor;
            }
            _ => (),
        }

        self.egui_winit.on_event(&self.egui_ctx, event)
    }

    pub fn end_frame(&mut self, window: &Window) {
        let full_output = self.egui_ctx.end_frame();

        self.egui_winit.handle_platform_output(
            &window.inner(),
            &self.egui_ctx,
            full_output.platform_output,
        );

        self.shapes = full_output.shapes;
        self.textures_delta.append(full_output.textures_delta);
    }

    pub fn paint(
        &mut self,
        command_buffer: ash::vk::CommandBuffer,
        swapchain_image_index: usize,
    ) -> anyhow::Result<(), RenderError> {
        let shapes = std::mem::take(&mut self.shapes);
        let textures_delta = std::mem::take(&mut self.textures_delta);

        let clipped_primitives = self.egui_ctx.tessellate(shapes);

        let index = swapchain_image_index;

        if textures_delta
            .set
            .contains_key(&egui::TextureId::Managed(0))
        {
            self.upload_font_texture(
                command_buffer,
                &textures_delta.set[&egui::TextureId::Managed(0)],
            )?;
        }

        let mut vertex_buffer_ptr = self.vertex_buffers[index].mapped;

        let vertex_buffer_ptr_end =
            unsafe { vertex_buffer_ptr.add(Self::vertex_buffer_size() as usize) };

        let mut index_buffer_ptr = self.index_buffers[index].mapped;

        let index_buffer_ptr_end =
            unsafe { index_buffer_ptr.add(Self::index_buffer_size() as usize) };

        unsafe {
            self.device.logical_device.cmd_begin_render_pass(
                command_buffer,
                &ash::vk::RenderPassBeginInfo::builder()
                    .render_pass(self.render_pass)
                    .framebuffer(self.framebuffers[index])
                    .clear_values(&[])
                    .render_area(
                        ash::vk::Rect2D::builder()
                            .extent(ash::vk::Extent2D {
                                width: self.physical_width,
                                height: self.physical_height,
                            })
                            .build(),
                    ),
                ash::vk::SubpassContents::INLINE,
            );
        }

        unsafe {
            self.pipeline.bind(command_buffer);

            self.vertex_buffers[index].bind_vertex(command_buffer);
            self.index_buffers[index].bind_index(command_buffer, ash::vk::IndexType::UINT32);

            self.device.logical_device.cmd_set_viewport(
                command_buffer,
                0,
                &[ash::vk::Viewport::builder()
                    .x(0.0)
                    .y(0.0)
                    .width(self.physical_width as f32)
                    .height(self.physical_height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0)
                    .build()],
            );

            let width_points = self.physical_width as f32 / self.scale_factor as f32;
            let height_points = self.physical_height as f32 / self.scale_factor as f32;

            self.device.logical_device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                ash::vk::ShaderStageFlags::VERTEX,
                0,
                bytemuck::bytes_of(&width_points),
            );

            self.device.logical_device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                ash::vk::ShaderStageFlags::VERTEX,
                std::mem::size_of_val(&width_points) as u32,
                bytemuck::bytes_of(&height_points),
            );
        }

        let mut vertex_base = 0;
        let mut index_base = 0;

        for cm in clipped_primitives.iter() {
            let mesh = match &cm.primitive {
                egui::epaint::Primitive::Mesh(mesh) => mesh,
                egui::epaint::Primitive::Callback(_) => {
                    continue;
                }
            };

            unsafe {
                if let egui::TextureId::User(id) = mesh.texture_id {
                    if let Some(descriptor_set) = self.user_textures[id as usize] {
                        self.device.logical_device.cmd_bind_descriptor_sets(
                            command_buffer,
                            ash::vk::PipelineBindPoint::GRAPHICS,
                            self.pipeline_layout,
                            0,
                            &[descriptor_set],
                            &[],
                        );
                    } else {
                        eprintln!(
                            "This UserTexture has already been unregistered: {:?}",
                            mesh.texture_id,
                        );
                        continue;
                    }
                } else {
                    self.device.logical_device.cmd_bind_descriptor_sets(
                        command_buffer,
                        ash::vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout,
                        0,
                        &[self.font_descriptor_sets[index]],
                        &[],
                    );
                }
            }

            if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                continue;
            }

            // self.vertex_buffers[index].write_to_buffer(&mesh.vertices);
            // self.index_buffers[index].write_to_buffer(&mesh.indices);

            let v_slice = &mesh.vertices;
            let v_size = std::mem::size_of_val(&v_slice[0]);
            let v_copy_size = v_slice.len() * v_size;

            let i_slice = &mesh.indices;
            let i_size = std::mem::size_of_val(&i_slice[0]);
            let i_copy_size = i_slice.len() * i_size;

            let vertex_buffer_ptr_next = unsafe { vertex_buffer_ptr.add(v_copy_size) };

            let index_buffer_ptr_next = unsafe { index_buffer_ptr.add(i_copy_size) };

            if vertex_buffer_ptr_next >= vertex_buffer_ptr_end
                || index_buffer_ptr_next >= index_buffer_ptr_end
            {
                panic!("egui paint out of memory");
            }

            // map memory
            unsafe {
                vertex_buffer_ptr.copy_from(v_slice.as_ptr() as *const c_void, v_copy_size);
                index_buffer_ptr.copy_from(i_slice.as_ptr() as *const c_void, i_copy_size);
            };

            vertex_buffer_ptr = vertex_buffer_ptr_next;
            index_buffer_ptr = index_buffer_ptr_next;

            unsafe {
                let min = cm.clip_rect.min;
                let min = egui::Pos2 {
                    x: min.x * self.scale_factor as f32,
                    y: min.y * self.scale_factor as f32,
                };

                let min = egui::Pos2 {
                    x: f32::clamp(min.x, 0.0, self.physical_width as f32),
                    y: f32::clamp(min.y, 0.0, self.physical_height as f32),
                };

                let max = cm.clip_rect.max;
                let max = egui::Pos2 {
                    x: max.x * self.scale_factor as f32,
                    y: max.y * self.scale_factor as f32,
                };

                let max = egui::Pos2 {
                    x: f32::clamp(max.x, min.x, self.physical_width as f32),
                    y: f32::clamp(max.y, min.y, self.physical_height as f32),
                };

                self.device.logical_device.cmd_set_scissor(
                    command_buffer,
                    0,
                    &[ash::vk::Rect2D::builder()
                        .offset(ash::vk::Offset2D {
                            x: min.x.round() as i32,
                            y: min.y.round() as i32,
                        })
                        .extent(ash::vk::Extent2D {
                            width: (max.x.round() - min.x) as u32,
                            height: (max.y.round() - min.y) as u32,
                        })
                        .build()],
                );

                self.device.logical_device.cmd_draw_indexed(
                    command_buffer,
                    mesh.indices.len() as u32,
                    1,
                    index_base,
                    vertex_base,
                    0,
                );
            }

            vertex_base += mesh.vertices.len() as i32;
            index_base += mesh.indices.len() as u32;
        }

        unsafe {
            self.device
                .logical_device
                .cmd_end_render_pass(command_buffer);
        }

        Ok(())
    }

    fn upload_font_texture(
        &mut self,
        command_buffer: ash::vk::CommandBuffer,
        delta: &egui::epaint::ImageDelta,
    ) -> anyhow::Result<(), RenderError> {
        let pixels: Vec<(u8, u8, u8, u8)> = match &delta.image {
            egui::ImageData::Color(image) => {
                assert_eq!(
                    image.width() * image.height(),
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );

                image.pixels.iter().map(|color| color.to_tuple()).collect()
            }
            egui::ImageData::Font(image) => {
                let gamma = 1.0;
                image
                    .srgba_pixels(gamma)
                    .map(|color| color.to_tuple())
                    .collect()
            }
        };

        let data = pixels
            .iter()
            .flat_map(|&r| vec![r.0, r.1, r.2, r.3])
            .collect::<Vec<u8>>();

        unsafe {
            if self.font_image_view.is_some() {
                drop(self.font_image_view.as_ref().unwrap());
            }

            self.device
                .logical_device
                .destroy_image(self.font_image.0, None);
            self.device
                .logical_device
                .free_memory(self.font_image.1, None);
        }

        self.font_image_staging_buffer = Buffer::new(
            self.device.clone(),
            (delta.image.width() * delta.image.height() * 4) as usize,
            ash::vk::BufferUsageFlags::TRANSFER_SRC,
            ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        self.font_image = {
            self.device.create_image_with_info(
                &ash::vk::ImageCreateInfo::builder()
                    .format(ash::vk::Format::R8G8B8A8_UNORM)
                    .initial_layout(ash::vk::ImageLayout::UNDEFINED)
                    .samples(ash::vk::SampleCountFlags::TYPE_1)
                    .tiling(ash::vk::ImageTiling::OPTIMAL)
                    .usage(
                        ash::vk::ImageUsageFlags::SAMPLED | ash::vk::ImageUsageFlags::TRANSFER_DST,
                    )
                    .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
                    .image_type(ash::vk::ImageType::TYPE_2D)
                    .mip_levels(1)
                    .array_layers(1)
                    .extent(ash::vk::Extent3D {
                        width: delta.image.width() as u32,
                        height: delta.image.height() as u32,
                        depth: 1,
                    }),
                ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?
        };

        self.font_image_view = Some(ImageView::new(
            self.device.clone(),
            self.font_image.0,
            ash::vk::Format::R8G8B8A8_UNORM,
            ash::vk::ImageAspectFlags::COLOR,
        )?);

        self.font_image_size = delta.image.size();

        self.font_descriptor_sets.clear();
        for descriptor_layout in self.descriptor_set_layouts.iter_mut() {
            let set = unsafe {
                DescriptorSetWriter::new(descriptor_layout.clone(), self.descriptor_pool.clone())
                    .write_image(
                        0,
                        &[ash::vk::DescriptorImageInfo::builder()
                            .image_view(self.font_image_view.as_ref().unwrap().view)
                            .image_layout(ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                            .sampler(self.sampler)
                            .build()],
                    )
                    .build()
                    .unwrap()
            };

            self.font_descriptor_sets.push(set);
        }

        unsafe {
            self.font_image_staging_buffer.map(0)?;
            self.font_image_staging_buffer.write_to_buffer(&data);

            self.device.logical_device.cmd_pipeline_barrier(
                command_buffer,
                ash::vk::PipelineStageFlags::HOST,
                ash::vk::PipelineStageFlags::TRANSFER,
                ash::vk::DependencyFlags::empty(),
                &[],
                &[],
                &[ash::vk::ImageMemoryBarrier::builder()
                    .image(self.font_image.0)
                    .subresource_range(ash::vk::ImageSubresourceRange {
                        aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .src_access_mask(ash::vk::AccessFlags::default())
                    .dst_access_mask(ash::vk::AccessFlags::TRANSFER_WRITE)
                    .old_layout(ash::vk::ImageLayout::UNDEFINED)
                    .new_layout(ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .build()],
            );

            self.device.logical_device.cmd_copy_buffer_to_image(
                command_buffer,
                self.font_image_staging_buffer.inner(),
                self.font_image.0,
                ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[ash::vk::BufferImageCopy::builder()
                    .image_subresource(
                        ash::vk::ImageSubresourceLayers::builder()
                            .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .layer_count(1)
                            .mip_level(0)
                            .build(),
                    )
                    .image_extent(ash::vk::Extent3D {
                        width: delta.image.width() as u32,
                        height: delta.image.height() as u32,
                        depth: 1,
                    })
                    .build()],
            );

            self.device.logical_device.cmd_pipeline_barrier(
                command_buffer,
                ash::vk::PipelineStageFlags::TRANSFER,
                ash::vk::PipelineStageFlags::ALL_GRAPHICS,
                ash::vk::DependencyFlags::empty(),
                &[],
                &[],
                &[ash::vk::ImageMemoryBarrier::builder()
                    .image(self.font_image.0)
                    .subresource_range(ash::vk::ImageSubresourceRange {
                        aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .src_access_mask(ash::vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(ash::vk::AccessFlags::SHADER_READ)
                    .old_layout(ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .build()],
            );
        }

        Ok(())
    }

    pub fn update_swapchain(
        &mut self,
        window: &Window,
        swapchain: &Swapchain,
        surface_format: ash::vk::Format,
    ) -> anyhow::Result<(), RenderError> {
        self.physical_width = window.inner().inner_size().width;
        self.physical_height = window.inner().inner_size().height;

        unsafe {
            self.device
                .logical_device
                .destroy_render_pass(self.render_pass, None);

            self.framebuffer_color_image_views
                .iter()
                .for_each(|iv| drop(iv));

            self.framebuffers
                .iter()
                .for_each(|f| self.device.logical_device.destroy_framebuffer(*f, None));
        }

        self.render_pass = Self::create_render_pass(&self.device, surface_format)?;

        self.pipeline = Self::create_pipeline(
            self.device.clone(),
            &self.render_pass,
            &self.pipeline_layout,
            5 * std::mem::size_of::<f32>() as u32,
        )?;

        let (framebuffer_color_image_views, framebuffers) = Self::create_framebuffers(
            self.device.clone(),
            window,
            swapchain,
            self.render_pass,
            surface_format,
        );

        self.framebuffer_color_image_views = framebuffer_color_image_views;
        self.framebuffers = framebuffers;

        Ok(())
    }

    fn create_render_pass(
        device: &Rc<Device>,
        surface_format: ash::vk::Format,
    ) -> anyhow::Result<ash::vk::RenderPass, RenderError> {
        Ok(unsafe {
            device.logical_device.create_render_pass(
                &ash::vk::RenderPassCreateInfo::builder()
                    .attachments(&[ash::vk::AttachmentDescription {
                        format: surface_format,
                        samples: ash::vk::SampleCountFlags::TYPE_1,
                        load_op: ash::vk::AttachmentLoadOp::LOAD,
                        store_op: ash::vk::AttachmentStoreOp::STORE,
                        stencil_load_op: ash::vk::AttachmentLoadOp::DONT_CARE,
                        stencil_store_op: ash::vk::AttachmentStoreOp::DONT_CARE,
                        initial_layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        final_layout: ash::vk::ImageLayout::PRESENT_SRC_KHR,
                        ..Default::default()
                    }])
                    .subpasses(&[ash::vk::SubpassDescription::builder()
                        .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
                        .color_attachments(&[ash::vk::AttachmentReference {
                            attachment: 0,
                            layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        }])
                        .build()])
                    .dependencies(&[ash::vk::SubpassDependency {
                        src_subpass: ash::vk::SUBPASS_EXTERNAL,
                        dst_subpass: 0,
                        src_stage_mask: ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        dst_stage_mask: ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        src_access_mask: ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                        dst_access_mask: ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                        ..Default::default()
                    }]),
                None,
            )?
        })
    }

    fn create_pipeline(
        device: Rc<Device>,
        render_pass: &ash::vk::RenderPass,
        pipeline_layout: &ash::vk::PipelineLayout,
        vertex_stride: u32,
    ) -> anyhow::Result<Rc<Pipeline>, RenderError> {
        Ok(Pipeline::start()
            .enable_alpha_blending()
            .binding_descriptions(vec![ash::vk::VertexInputBindingDescription {
                binding: 0,
                stride: vertex_stride,
                input_rate: ash::vk::VertexInputRate::VERTEX,
            }])
            .attribute_descriptions(vec![
                ash::vk::VertexInputAttributeDescription {
                    binding: 0,
                    location: 0,
                    format: ash::vk::Format::R32G32_SFLOAT,
                    offset: 0,
                },
                ash::vk::VertexInputAttributeDescription {
                    binding: 0,
                    location: 1,
                    format: ash::vk::Format::R32G32_SFLOAT,
                    offset: 8,
                },
                ash::vk::VertexInputAttributeDescription {
                    binding: 0,
                    location: 2,
                    format: ash::vk::Format::R8G8B8A8_UNORM,
                    offset: 16,
                },
            ])
            .build(
                device,
                "client/shaders/egui.vert.spv", // needs fixing for release mode
                "client/shaders/egui.frag.spv", // needs fixing for release mode
                render_pass,
                pipeline_layout,
            )?)
    }

    fn create_framebuffers(
        device: Rc<Device>,
        window: &Window,
        swapchain: &Swapchain,
        render_pass: ash::vk::RenderPass,
        surface_format: ash::vk::Format,
    ) -> (Vec<Rc<ImageView>>, Vec<ash::vk::Framebuffer>) {
        let framebuffer_color_image_views = swapchain
            .swapchain_images
            .iter()
            .map(
                |swapchain_image| {
                    ImageView::new(
                        device.clone(),
                        swapchain_image.clone(),
                        surface_format,
                        ash::vk::ImageAspectFlags::COLOR,
                    )
                    .expect("Failed to create image view")
                }, // make better?
            )
            .collect::<Vec<_>>();

        let framebuffers = framebuffer_color_image_views
            .iter()
            .map(|image_view| image_view.view)
            .map(|view| unsafe {
                let attachments = &[view];
                device
                    .logical_device
                    .create_framebuffer(
                        &ash::vk::FramebufferCreateInfo::builder()
                            .render_pass(render_pass)
                            .attachments(attachments)
                            .width(window.inner().inner_size().width)
                            .height(window.inner().inner_size().height)
                            .layers(1),
                        None,
                    )
                    .expect("Failed to create framebuffer")
            })
            .collect::<Vec<_>>();

        (framebuffer_color_image_views, framebuffers)
    }
}

impl Drop for EGuiIntegration {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical_device
                .destroy_image(self.font_image.0, None);
            self.device
                .logical_device
                .free_memory(self.font_image.1, None);

            self.framebuffers
                .iter()
                .for_each(|f| self.device.logical_device.destroy_framebuffer(*f, None));

            self.device
                .logical_device
                .destroy_render_pass(self.render_pass, None);
            self.device
                .logical_device
                .destroy_sampler(self.sampler, None);

            self.device
                .logical_device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
