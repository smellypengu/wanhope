use std::rc::Rc;

use crate::{game_object::GameObject, vulkan::{Vertex, Align16, Device, Pipeline, RenderError}, camera::Camera};

#[derive(Debug)]
#[repr(C)]
pub struct SimplePushConstantData {
    transform: glam::Mat4,
    color: glam::Vec3,
}

impl SimplePushConstantData {
    pub unsafe fn as_bytes(&self) -> &[u8] {
        let size_in_bytes = std::mem::size_of::<Self>();
        let size_in_u8 = size_in_bytes / std::mem::size_of::<u8>();
        let start_ptr = self as *const Self as *const u8;
        std::slice::from_raw_parts(start_ptr, size_in_u8)
    }
}

pub struct SimpleRenderSystem {
    device: Rc<Device>,
    pipeline: Rc<Pipeline>,
    pipeline_layout: ash::vk::PipelineLayout,
}

impl SimpleRenderSystem {
    pub fn new(
        device: Rc<Device>,
        render_pass: &ash::vk::RenderPass,
    ) -> anyhow::Result<Self, RenderError> {
        let pipeline_layout = Self::create_pipeline_layout(
            &device.logical_device,
        )?;

        let pipeline = Self::create_pipeline(
            device.clone(),
            render_pass,
            &pipeline_layout,
        )?;

        Ok(Self {
            device,
            pipeline,
            pipeline_layout,
        })
    }

    fn create_pipeline(
        device: Rc<Device>,
        render_pass: &ash::vk::RenderPass,
        pipeline_layout: &ash::vk::PipelineLayout,
    ) -> anyhow::Result<Rc<Pipeline>, RenderError> {
        assert!(
            pipeline_layout != &ash::vk::PipelineLayout::null(),
            "Cannot create pipeline before pipeline layout"
        );

        let pipeline = Pipeline::start()
            .binding_descriptions(Vertex::binding_descriptions())
            .attribute_descriptions(Vertex::attribute_descriptions())
            .build(
                device.clone(),
                "shaders/simple_shader.vert.spv",
                "shaders/simple_shader.frag.spv",
                &render_pass,
                &pipeline_layout,
            )?;

        Ok(pipeline)
    }

    fn create_pipeline_layout(
        logical_device: &ash::Device,
    ) -> anyhow::Result<ash::vk::PipelineLayout, RenderError> {
        let push_constant_range = [ash::vk::PushConstantRange {
            stage_flags: ash::vk::ShaderStageFlags::VERTEX | ash::vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<SimplePushConstantData>() as u32,
        }];

        let pipeline_layout_info = ash::vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(&push_constant_range);

        Ok(unsafe {
            logical_device.create_pipeline_layout(&pipeline_layout_info, None)?
        })
    }

    pub fn render_game_objects(
        &self,
        command_buffer: ash::vk::CommandBuffer,
        game_objects: &mut Vec<GameObject>,
        camera: &Camera,
    ) {
        unsafe {
            self.pipeline.bind(command_buffer);
        }

        let projection_view = camera.projection_matrix * camera.view_matrix;

        for obj in game_objects.iter_mut() {
            match &obj.model {
                Some(model) => {
                    let push = SimplePushConstantData {
                        transform: projection_view * obj.transform.mat4(),
                        color: obj.color,
                    };
    
                    unsafe {
                        let push_ptr = push.as_bytes();
    
                        self.device.logical_device.cmd_push_constants(
                            command_buffer,
                            self.pipeline_layout,
                            ash::vk::ShaderStageFlags::VERTEX | ash::vk::ShaderStageFlags::FRAGMENT,
                            0,
                            push_ptr,
                        );
    
                        model.bind(command_buffer);
                        model.draw(&self.device.logical_device, command_buffer);
                    }
                },
                None => { },
            }
        }
    }
}

impl Drop for SimpleRenderSystem {
    fn drop(&mut self) {
        log::debug!("Dropping simple render system");

        unsafe {
            self.device.logical_device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
