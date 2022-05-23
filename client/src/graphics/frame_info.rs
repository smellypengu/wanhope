use std::collections::HashMap;

use crate::game_object::GameObject;

use super::Camera;

pub const MAX_LIGHTS: usize = 10;

#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PointLight {
    pub position: glam::Vec4, // ignore w
    pub color: glam::Vec4,    // w is intensity
}

#[derive(PartialEq)]
#[repr(C)]
pub struct GlobalUbo {
    pub projection: glam::Mat4,
    pub view: glam::Mat4,
    pub inverse_view: glam::Mat4,
    pub ambient_light_color: glam::Vec4, // w is intensity
    pub point_lights: [PointLight; MAX_LIGHTS],
    pub num_lights: u32,
}

pub struct FrameInfo<'a> {
    pub frame_index: usize,
    pub frame_time: f32,
    pub command_buffer: ash::vk::CommandBuffer,
    pub camera: Camera,
    pub global_descriptor_set: ash::vk::DescriptorSet,
    pub game_objects: &'a mut HashMap<u8, GameObject>,
}
