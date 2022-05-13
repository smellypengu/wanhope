use glam::Vec4Swizzles;

pub struct CameraBuilder {
    pub projection_matrix: glam::Mat4,
    pub view_matrix: glam::Mat4,
    pub inverse_view_matrix: glam::Mat4,
}

pub struct Camera {
    pub projection_matrix: glam::Mat4,
    pub view_matrix: glam::Mat4,
    pub inverse_view_matrix: glam::Mat4,
}

impl CameraBuilder {
    pub fn set_orthographic_projection<'a>(
        &'a mut self,
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        near: f32,
        far: f32,
    ) -> &'a mut CameraBuilder {
        self.projection_matrix = glam::Mat4::orthographic_rh(left, right, bottom, top, near, far);

        self
    }

    pub fn set_perspective_projection<'a>(
        &'a mut self,
        fovy: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> &'a mut Self {
        assert!((aspect - std::f32::EPSILON).abs() > 0.0);

        self.projection_matrix = glam::Mat4::perspective_rh(fovy, aspect, near, far);

        self
    }

    pub fn set_view_xyz<'a>(
        &'a mut self,
        position: glam::Vec3,
        rotation: glam::Vec3,
    ) -> &'a mut CameraBuilder {
        let dir = glam::Vec3::new(
            rotation.y.cos() * rotation.x.sin(),
            rotation.y.sin(),
            rotation.y.cos() * rotation.x.cos(),
        );

        let view_matrix = glam::Mat4::look_at_rh(position, position - dir, glam::Vec3::new(0.0, 1.0, 0.0));

        self.view_matrix = view_matrix;

        self.inverse_view_matrix = view_matrix.inverse();

        self
    }

    pub fn build(&self) -> Camera {
        Camera {
            projection_matrix: self.projection_matrix,
            view_matrix: self.view_matrix,
            inverse_view_matrix: self.inverse_view_matrix,
        }
    }
}

impl Camera {
    pub fn new() -> CameraBuilder {
        CameraBuilder {
            projection_matrix: glam::Mat4::IDENTITY,
            view_matrix: glam::Mat4::IDENTITY,
            inverse_view_matrix: glam::Mat4::IDENTITY,
        }
    }

    pub fn position(&self) -> glam::Vec3 {
        self.inverse_view_matrix.col(3).xyz()
    }
}
