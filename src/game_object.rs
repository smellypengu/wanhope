use std::rc::Rc;

use crate::vulkan::Model;

pub struct TransformComponent {
    pub translation: glam::Vec3,
    pub scale: glam::Vec3,
    pub rotation: glam::Vec3,
}

impl TransformComponent {
    pub fn mat4(&self) -> glam::Mat4 {
        let quat = glam::Quat::from_euler(glam::EulerRot::XYZ, self.rotation.x, self.rotation.y, self.rotation.z);

        glam::Mat4::from_scale_rotation_translation(self.scale, quat, self.translation)
    }
}

pub struct GameObject {
    pub model: Option<Rc<Model>>,
    pub color: glam::Vec3,
    pub transform: TransformComponent,
}

impl GameObject {
    pub fn new(
        model: Option<Rc<Model>>,
        color: Option<glam::Vec3>,
        transform: Option<TransformComponent>,
    ) -> GameObject {
        let color = match color {
            Some(c) => c,
            None => glam::vec3(0.0, 0.0, 0.0),
        };

        let transform = match transform {
            Some(t) => t,
            None => TransformComponent {
                translation: glam::Vec3::ZERO,
                scale: glam::Vec3::ONE,
                rotation: glam::Vec3::ZERO,
            }
        };

        Self {
            model,
            color,
            transform,
        }
    }
}
