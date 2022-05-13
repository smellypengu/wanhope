use std::rc::Rc;

use crate::vulkan::{Device, RenderError};

pub struct DescriptorPoolBuilder {
    device: Rc<Device>,
    pool_sizes: Vec<ash::vk::DescriptorPoolSize>,
    max_sets: u32,
    pool_flags: ash::vk::DescriptorPoolCreateFlags,
}

impl DescriptorPoolBuilder {
    pub fn pool_size(
        mut self,
        descriptor_type: ash::vk::DescriptorType,
        count: u32,
    ) -> Self {
        self.pool_sizes.push(
            ash::vk::DescriptorPoolSize {
                ty: descriptor_type,
                descriptor_count: count,
            }
        );

        self
    }

    pub fn pool_flags(
        mut self,
        flags: ash::vk::DescriptorPoolCreateFlags,
    ) -> Self {
        self.pool_flags = flags;

        self
    }

    pub fn max_sets(
        mut self,
        max_sets: u32,
    ) -> Self {
        self.max_sets = max_sets;

        self
    }

    pub unsafe fn build(self) -> anyhow::Result<Rc<DescriptorPool>, RenderError> {
        let DescriptorPoolBuilder {
            device,
            pool_sizes,
            max_sets,
            pool_flags,
        } = self;
        
        let pool_info = ash::vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(max_sets)
            .flags(pool_flags);

        let pool = device.logical_device.create_descriptor_pool(&pool_info, None)?;

        Ok(Rc::new(DescriptorPool {
            device,
            pool,
        }))
    }
}

pub struct DescriptorPool {
    pub device: Rc<Device>,
    pub pool: ash::vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn new(
        device: Rc<Device>,
    ) -> DescriptorPoolBuilder {
        DescriptorPoolBuilder {
            device,
            pool_sizes: Vec::new(),
            max_sets: 1000,
            pool_flags: ash::vk::DescriptorPoolCreateFlags::empty(),
        }
    }

    pub unsafe fn allocate_descriptor(
        &self,
        layouts: &[ash::vk::DescriptorSetLayout],
    ) -> anyhow::Result<ash::vk::DescriptorSet, RenderError> {
        let alloc_info = ash::vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.pool)
            .set_layouts(layouts)
            .build();

        Ok(self.device.logical_device.allocate_descriptor_sets(&alloc_info,)?[0])
    }

    pub unsafe fn free_descriptors(
        &self,
        descriptors: &Vec<ash::vk::DescriptorSet>
    ) -> anyhow::Result<(), RenderError> {
        Ok(self.device.logical_device.free_descriptor_sets(
            self.pool,
            descriptors,
        )?)
    }

    pub unsafe fn reset_pool(&self) -> anyhow::Result<(), RenderError> {
        Ok(self.device.logical_device.reset_descriptor_pool(
            self.pool,
            ash::vk::DescriptorPoolResetFlags::empty(),
        )?)
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        log::debug!("Dropping vulkan descriptor pool");

        unsafe {
            self.device.logical_device.destroy_descriptor_pool(self.pool, None)
        }
    }
}
