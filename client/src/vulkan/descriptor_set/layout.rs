use std::{collections::HashMap, rc::Rc};

use crate::vulkan::{Device, RenderError};

pub struct DescriptorSetLayoutBuilder {
    device: Rc<Device>,
    bindings: HashMap<u32, ash::vk::DescriptorSetLayoutBinding>,
}

pub struct DescriptorSetLayout {
    device: Rc<Device>,
    layout: ash::vk::DescriptorSetLayout,
    pub bindings: HashMap<u32, ash::vk::DescriptorSetLayoutBinding>,
}

impl DescriptorSetLayoutBuilder {
    pub fn add_binding(
        mut self,
        binding: u32,
        descriptor_type: ash::vk::DescriptorType,
        stage_flags: ash::vk::ShaderStageFlags,
        descriptor_count: u32,
    ) -> Self {
        assert_eq!(
            self.bindings.keys().filter(|&b| *b == binding).count(),
            0,
            "Binding already in use",
        );

        let layout_binding = ash::vk::DescriptorSetLayoutBinding {
            binding,
            descriptor_type,
            descriptor_count,
            stage_flags,
            ..Default::default()
        };

        self.bindings.insert(binding, layout_binding);

        self
    }

    pub unsafe fn build(self) -> anyhow::Result<Rc<DescriptorSetLayout>, RenderError> {
        let DescriptorSetLayoutBuilder {
            device,
            bindings
        } = self;

        let mut set_layout_bindings = Vec::new();
        for binding in bindings.values() {
            set_layout_bindings.push(*binding);
        }

        let layout_info = ash::vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&set_layout_bindings);

        let layout = device.logical_device.create_descriptor_set_layout(&layout_info, None)?;

        Ok(Rc::new(DescriptorSetLayout {
            device,
            layout,
            bindings,
        }))
    }
}

impl DescriptorSetLayout {
    pub fn new(
        device: Rc<Device>,
    ) -> DescriptorSetLayoutBuilder {
        DescriptorSetLayoutBuilder {
            device,
            bindings: HashMap::new(),
        }
    }

    #[inline]
    pub fn inner(&self) -> ash::vk::DescriptorSetLayout {
        self.layout
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        log::debug!("Dropping vulkan descriptor set layout");

        unsafe {
            self.device.logical_device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}

