use glam::Vec4Swizzles;

use super::Camera;

pub struct Ray {
    pub origin: glam::Vec3,
    pub dir: glam::Vec3,
}

impl Ray {
    pub fn from_screenspace(
        cursor_pos: glam::Vec2,
        window_size: glam::Vec2,
        camera: &Camera,
    ) -> Option<Self> {
        // normalized device coordinates
        let point = (cursor_pos / window_size) * 2.0 - glam::vec2(1.0, 1.0);

        // 4d homogeneous clip coordinates
        let ray_clip = glam::vec4(point.x, point.y, -1.0, 1.0);

        // 4d eye (camera) coordinates
        let ray_eye = camera.projection_matrix.inverse() * ray_clip; // TODO: precalculate inverse projection_matrix?
        let ray_eye = glam::vec4(ray_eye.x, ray_eye.y, -1.0, 0.0);

        // 3d world coordinates
        let ray_world = (camera.inverse_view_matrix * ray_eye).xyz();

        let origin = (camera.inverse_view_matrix * glam::Vec4::W).xyz();

        Some(Self {
            origin,
            dir: ray_world.normalize(),
        })
    }
}
