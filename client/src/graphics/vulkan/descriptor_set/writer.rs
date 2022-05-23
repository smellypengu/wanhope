use std::rc::Rc;

use super::{DescriptorPool, DescriptorSetLayout};

pub struct DescriptorSetWriter {
    set_layout: Rc<DescriptorSetLayout>,
    pool: Rc<DescriptorPool>,
    writes: Vec<ash::vk::WriteDescriptorSet>,
}

impl DescriptorSetWriter {
    pub fn new(set_layout: Rc<DescriptorSetLayout>, pool: Rc<DescriptorPool>) -> Self {
        DescriptorSetWriter {
            set_layout,
            pool,
            writes: Vec::new(),
        }
    }

    pub fn write_to_buffer(
        mut self,
        binding: u32,
        buffer_info: &[ash::vk::DescriptorBufferInfo],
    ) -> Self {
        assert_eq!(
            self.set_layout
                .bindings
                .keys()
                .filter(|&b| *b == binding)
                .count(),
            1,
            "Layout does not contain specified binding",
        );

        let binding_description = self.set_layout.bindings[&binding];

        assert_eq!(
            binding_description.descriptor_count, 1,
            "Binding single descriptor info, but binding expects multiple",
        );

        let write = ash::vk::WriteDescriptorSet::builder()
            .descriptor_type(binding_description.descriptor_type)
            .dst_binding(binding)
            .buffer_info(buffer_info)
            .build();

        self.writes.push(write);

        self
    }

    pub fn write_image(
        mut self,
        binding: u32,
        image_info: &[ash::vk::DescriptorImageInfo],
    ) -> Self {
        assert!(
            self.set_layout
                .bindings
                .keys()
                .filter(|&b| *b == binding)
                .count()
                == 1,
            "Layout does not contain specified binding",
        );

        let binding_description = self.set_layout.bindings[&binding];

        assert_eq!(
            binding_description.descriptor_count, 1,
            "Binding single descriptor info, but binding expects multiple",
        );

        let write = ash::vk::WriteDescriptorSet::builder()
            .descriptor_type(binding_description.descriptor_type)
            .dst_binding(binding)
            .image_info(image_info)
            .build();

        self.writes.push(write);

        self
    }

    pub unsafe fn build(&mut self) -> Option<ash::vk::DescriptorSet> {
        let result = self.pool.allocate_descriptor(&[self.set_layout.inner()]);

        if result.is_err() {
            return None;
        }

        let set = self.overwrite(result.unwrap());
        Some(set)
    }

    pub unsafe fn overwrite(&mut self, set: ash::vk::DescriptorSet) -> ash::vk::DescriptorSet {
        for mut write in self.writes.iter_mut() {
            write.dst_set = set;
        }

        self.pool
            .device
            .logical_device
            .update_descriptor_sets(&self.writes, &[]);

        set
    }
}
