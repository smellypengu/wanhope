use std::rc::Rc;

use crate::vulkan::Model;

pub struct TransformComponent {
    pub translation: glam::Vec2,
}

impl TransformComponent {
    pub fn mat2(&self) -> glam::Mat2 {
        glam::Mat2::IDENTITY
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
                translation: glam::vec2(0.0, 0.0),
            }
        };

        Self {
            model,
            color,
            transform,
        }
    }
}
