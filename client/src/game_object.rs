use std::rc::Rc;

use crate::vulkan::Model;

pub struct TransformComponent {
    pub translation: glam::Vec3,
    pub scale: glam::Vec3,
    pub rotation: glam::Vec3,
}

impl TransformComponent {
    pub fn mat4(&self) -> glam::Mat4 {
        let quat = glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
        );

        glam::Mat4::from_scale_rotation_translation(self.scale, quat, self.translation)
    }

    pub fn normal_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale(1.0 / self.scale) // mat4 instead of mat3 for alignment
    }
}

pub struct PointLightComponent {
    pub light_intensity: f32,
}

static mut CURRENT_ID: u8 = 0;

pub struct GameObject {
    pub id: u8,
    pub model: Option<Rc<Model>>,
    pub color: glam::Vec3,
    pub transform: TransformComponent,
    pub point_light: Option<PointLightComponent>,
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
            },
        };

        // perhaps get fix the unsafe
        let id = unsafe { CURRENT_ID };

        unsafe {
            CURRENT_ID += 1;
        }

        Self {
            id,
            model,
            color,
            transform,
            point_light: None,
        }
    }

    pub fn make_point_light(intensity: f32, radius: f32, color: glam::Vec3) -> Self {
        let mut game_object = Self::new(
            None,
            Some(color),
            Some(TransformComponent {
                translation: glam::Vec3::ZERO,
                scale: glam::vec3(radius, 0.0, 0.0),
                rotation: glam::Vec3::ZERO,
            }),
        );

        game_object.point_light = Some(PointLightComponent {
            light_intensity: intensity,
        });

        game_object
    }
}
