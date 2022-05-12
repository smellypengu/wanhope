use crate::camera::Camera;

pub struct FrameInfo {
    pub frame_index: usize,
    pub frame_time: f32,
    pub command_buffer: ash::vk::CommandBuffer,
    pub camera: Camera,
    pub global_descriptor_set: ash::vk::DescriptorSet,
}
