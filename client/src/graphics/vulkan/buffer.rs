use std::{ffi::c_void, marker::PhantomData, rc::Rc};

use super::{Device, RenderError};

pub struct Buffer<T>
where
    T: PartialEq,
{
    device: Rc<Device>,
    buffer: ash::vk::Buffer,
    memory: ash::vk::DeviceMemory,
    pub mapped: *mut c_void,
    capacity: usize,
    coherent: bool,

    _p: PhantomData<T>,
}

impl<T> Buffer<T>
where
    T: PartialEq,
{
    pub unsafe fn new(
        device: Rc<Device>,
        size: usize,
        usage_flags: ash::vk::BufferUsageFlags,
        memory_property_flags: ash::vk::MemoryPropertyFlags,
    ) -> anyhow::Result<Self, RenderError> {
        let byte_len = std::mem::size_of::<T>() * size;

        let (buffer, memory, coherent) =
            device.create_buffer(byte_len as u64, usage_flags, memory_property_flags)?;

        Ok(Self {
            device,
            buffer,
            memory,
            mapped: std::ptr::null_mut(),
            capacity: size,
            coherent,

            _p: PhantomData {},
        })
    }

    pub unsafe fn bind_vertex(&self, command_buffer: ash::vk::CommandBuffer) {
        self.device
            .logical_device
            .cmd_bind_vertex_buffers(command_buffer, 0, &[self.buffer], &[0])
    }

    pub unsafe fn bind_index(
        &self,
        command_buffer: ash::vk::CommandBuffer,
        index_type: ash::vk::IndexType,
    ) {
        self.device
            .logical_device
            .cmd_bind_index_buffer(command_buffer, self.buffer, 0, index_type)
    }

    pub unsafe fn map(&mut self, element_offset: usize) -> anyhow::Result<(), RenderError> {
        let size = self.capacity - element_offset;

        let mem_size = (std::mem::size_of::<T>() * size) as u64;
        let mem_offset = (std::mem::size_of::<T>() * element_offset) as u64;

        Ok(self.mapped = self.device.logical_device.map_memory(
            self.memory,
            mem_offset,
            mem_size,
            ash::vk::MemoryMapFlags::empty(),
        )?)
    }

    pub unsafe fn unmap(&mut self) {
        if !self.mapped.is_null() {
            self.device.logical_device.unmap_memory(self.memory);

            self.mapped = std::ptr::null_mut();
        }
    }

    pub unsafe fn write_to_buffer(&mut self, elements: &[T]) {
        assert!(!self.mapped.is_null(), "Cannot copy to unmapped buffer",);

        elements
            .as_ptr()
            .copy_to_nonoverlapping(self.mapped as *mut _, elements.len());
    }

    pub unsafe fn flush(&self) -> anyhow::Result<(), RenderError> {
        if self.coherent {
            return Ok(());
        }

        let mapped_range = [ash::vk::MappedMemoryRange {
            memory: self.memory,
            offset: 0,
            size: ash::vk::WHOLE_SIZE,
            ..Default::default()
        }];

        Ok(self
            .device
            .logical_device
            .flush_mapped_memory_ranges(&mapped_range)?)
    }

    #[inline]
    pub fn descriptor_info(&self) -> ash::vk::DescriptorBufferInfo {
        ash::vk::DescriptorBufferInfo {
            buffer: self.buffer,
            offset: 0,
            range: ash::vk::WHOLE_SIZE,
        }
    }

    #[inline]
    pub fn inner(&self) -> ash::vk::Buffer {
        self.buffer
    }
}

impl<T> Drop for Buffer<T>
where
    T: PartialEq,
{
    fn drop(&mut self) {
        log::debug!("Dropping vulkan buffer");

        unsafe {
            self.unmap();
            self.device.logical_device.destroy_buffer(self.buffer, None);
            self.device.logical_device.free_memory(self.memory, None);
        }
    }
}
