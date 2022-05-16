use std::{rc::Rc, ffi::CString};

use super::{Device, ShaderModule, RenderError};

struct PipelineInfo {
    binding_descriptions: Vec<ash::vk::VertexInputBindingDescription>,
    attribute_descriptions: Vec<ash::vk::VertexInputAttributeDescription>,

    input_assembly_info: ash::vk::PipelineInputAssemblyStateCreateInfo,
    viewport_info: ash::vk::PipelineViewportStateCreateInfo,
    rasterization_info: ash::vk::PipelineRasterizationStateCreateInfo,
    multisample_info: ash::vk::PipelineMultisampleStateCreateInfo,
    color_blend_attachments: Vec<ash::vk::PipelineColorBlendAttachmentState>,
    color_blend_info: ash::vk::PipelineColorBlendStateCreateInfo,
    depth_stencil_info: ash::vk::PipelineDepthStencilStateCreateInfo,
    dynamic_state_enables: Vec<ash::vk::DynamicState>,
    dynamic_state_info: ash::vk::PipelineDynamicStateCreateInfo,
    subpass: u32,
}

pub struct PipelineBuilder {
    pipeline_info: PipelineInfo,
}

impl PipelineBuilder {
    pub fn start() -> Self {
        let input_assembly_info = ash::vk::PipelineInputAssemblyStateCreateInfo {
            topology: ash::vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: ash::vk::FALSE,
            ..Default::default()
        };

        let viewport_info = ash::vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            // p_viewports: ,
            scissor_count: 1,
            // p_scissors: ,
            ..Default::default()
        };

        let rasterization_info = ash::vk::PipelineRasterizationStateCreateInfo {
            depth_clamp_enable: ash::vk::FALSE,
            rasterizer_discard_enable: ash::vk::FALSE,
            polygon_mode: ash::vk::PolygonMode::FILL,
            cull_mode: ash::vk::CullModeFlags::NONE,
            front_face: ash::vk::FrontFace::CLOCKWISE,
            depth_bias_enable: ash::vk::FALSE,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
            ..Default::default()
        };

        let multisample_info = ash::vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: ash::vk::SampleCountFlags::TYPE_1,
            sample_shading_enable: ash::vk::FALSE,
            min_sample_shading: 1.0,
            // p_sample_mask: ,
            alpha_to_coverage_enable: ash::vk::FALSE,
            alpha_to_one_enable: ash::vk::FALSE,
            ..Default::default()
        };

        let color_blend_attachments = vec![ash::vk::PipelineColorBlendAttachmentState {
            blend_enable: ash::vk::FALSE,
            src_color_blend_factor: ash::vk::BlendFactor::ONE,
            dst_color_blend_factor: ash::vk::BlendFactor::ZERO,
            color_blend_op: ash::vk::BlendOp::ADD,
            src_alpha_blend_factor: ash::vk::BlendFactor::ONE,
            dst_alpha_blend_factor: ash::vk::BlendFactor::ZERO,
            alpha_blend_op: ash::vk::BlendOp::ADD,
            color_write_mask: ash::vk::ColorComponentFlags::RGBA,
        }];

        let color_blend_info = ash::vk::PipelineColorBlendStateCreateInfo {
            logic_op_enable: ash::vk::FALSE,
            logic_op: ash::vk::LogicOp::COPY,
            attachment_count: 1,
            p_attachments: color_blend_attachments.as_ptr(),
            blend_constants: [0.0, 0.0 ,0.0, 0.0],
            ..Default::default()
        };

        let depth_stencil_info = ash::vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: ash::vk::TRUE,
            depth_write_enable: ash::vk::TRUE,
            depth_compare_op: ash::vk::CompareOp::LESS,
            depth_bounds_test_enable: ash::vk::FALSE,
            stencil_test_enable: ash::vk::FALSE,
            // front: ,
            // back: ,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
            ..Default::default()
        };

        let dynamic_state_enables = vec![
            ash::vk::DynamicState::VIEWPORT,
            ash::vk::DynamicState::SCISSOR,
        ];

        let dynamic_state_info = ash::vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: dynamic_state_enables.len() as u32,
            p_dynamic_states: dynamic_state_enables.as_ptr(),
            ..Default::default()
        };

        let pipeline_info = PipelineInfo {
            binding_descriptions: vec![],
            attribute_descriptions: vec![],

            input_assembly_info,
            viewport_info,
            rasterization_info,
            multisample_info,
            color_blend_attachments,
            color_blend_info,
            depth_stencil_info,
            dynamic_state_enables,
            dynamic_state_info,
            subpass: 0,
        };

        Self {
            pipeline_info,
        }
    }

    pub fn binding_descriptions(
        mut self,
        binding_descriptions: Vec<ash::vk::VertexInputBindingDescription>,
    ) -> Self {
        self.pipeline_info.binding_descriptions = binding_descriptions;

        self
    }

    pub fn attribute_descriptions(
        mut self,
        attribute_descriptions: Vec<ash::vk::VertexInputAttributeDescription>,
    ) -> Self {
        self.pipeline_info.attribute_descriptions = attribute_descriptions;

        self
    }

    pub fn enable_alpha_blending(mut self) -> PipelineBuilder {
        self.pipeline_info.color_blend_attachments = vec![ash::vk::PipelineColorBlendAttachmentState {
            blend_enable: ash::vk::TRUE,
            src_color_blend_factor: ash::vk::BlendFactor::SRC_ALPHA,
            dst_color_blend_factor: ash::vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: ash::vk::BlendOp::ADD,
            src_alpha_blend_factor: ash::vk::BlendFactor::ONE,
            dst_alpha_blend_factor: ash::vk::BlendFactor::ZERO,
            alpha_blend_op: ash::vk::BlendOp::ADD,
            color_write_mask: ash::vk::ColorComponentFlags::RGBA,
        }];

        self.pipeline_info.color_blend_info.p_attachments = self.pipeline_info.color_blend_attachments.as_ptr();

        self
    }

    pub fn build(
        &self,
        device: Rc<Device>,
        vert_file_path: &str,
        frag_file_path: &str,
        render_pass: &ash::vk::RenderPass,
        pipeline_layout: &ash::vk::PipelineLayout,
    ) -> anyhow::Result<Rc<Pipeline>, RenderError> {
        let vert_shader_module = ShaderModule::new(
            device.clone(),
            vert_file_path,
        )?;

        let frag_shader_module = ShaderModule::new(
            device.clone(),
            frag_file_path,
        )?;

        let entry_point_name = CString::new("main").unwrap();

        let vert_shader_stage_info = ash::vk::PipelineShaderStageCreateInfo {
            stage: ash::vk::ShaderStageFlags::VERTEX,
            module: vert_shader_module.inner(),
            p_name: entry_point_name.as_ptr() as _,
            ..Default::default()
        };

        let frag_shader_stage_info = ash::vk::PipelineShaderStageCreateInfo {
            stage: ash::vk::ShaderStageFlags::FRAGMENT,
            module: frag_shader_module.inner(),
            p_name: entry_point_name.as_ptr() as _,
            ..Default::default()
        };

        let shader_stages = [
            vert_shader_stage_info, 
            frag_shader_stage_info,
        ];

        let vertex_input_info = ash::vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&self.pipeline_info.binding_descriptions)
            .vertex_attribute_descriptions(&self.pipeline_info.attribute_descriptions);

        let pipeline_info = ash::vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&self.pipeline_info.input_assembly_info)
            .viewport_state(&self.pipeline_info.viewport_info)
            .rasterization_state(&self.pipeline_info.rasterization_info)
            .multisample_state(&self.pipeline_info.multisample_info)
            .color_blend_state(&self.pipeline_info.color_blend_info)
            .depth_stencil_state(&self.pipeline_info.depth_stencil_info)
            .dynamic_state(&self.pipeline_info.dynamic_state_info)
            .layout(*pipeline_layout)
            .render_pass(*render_pass)
            .subpass(self.pipeline_info.subpass)
            .base_pipeline_index(-1)
            .base_pipeline_handle(ash::vk::Pipeline::null());

        let graphics_pipeline = unsafe {
            device.logical_device.create_graphics_pipelines(
                ash::vk::PipelineCache::null(),
                std::slice::from_ref(&pipeline_info),
                None
            ).map_err(|e| log::error!("Unable to create graphics pipeline: {:?}", e)).unwrap()[0] // fix unwrap?
        };

        Ok(Rc::new(Pipeline {
            device,
            graphics_pipeline,
            vert_shader_module,
            frag_shader_module,
        }))
    }
}

pub struct Pipeline {
    device: Rc<Device>,
    graphics_pipeline: ash::vk::Pipeline,
    vert_shader_module: Rc<ShaderModule>,
    frag_shader_module: Rc<ShaderModule>,
}

impl Pipeline {
    pub fn start() -> PipelineBuilder {
        PipelineBuilder::start()
    }

    pub unsafe fn bind(&self, command_buffer: ash::vk::CommandBuffer) {
        self.device.logical_device.cmd_bind_pipeline(
            command_buffer,
            ash::vk::PipelineBindPoint::GRAPHICS,
            self.graphics_pipeline,
        );
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        log::debug!("Dropping vulkan pipeline");
        
        unsafe {
            self.device.logical_device.destroy_pipeline(self.graphics_pipeline, None);
        }
    }
}
