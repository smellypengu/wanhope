use std::f32::{consts::PI, EPSILON};

use winit::event::VirtualKeyCode;

use crate::{game_object::GameObject, Input};

pub struct KeyboardMovementController {
    move_speed: f32,
    look_speed: f32,
}

impl KeyboardMovementController {
    pub fn new(move_speed: Option<f32>, look_speed: Option<f32>) -> Self {
        let move_speed = match move_speed {
            Some(speed) => speed,
            None => 3.0,
        };

        let look_speed = match look_speed {
            Some(speed) => speed,
            None => 3.0,
        };

        Self {
            move_speed,
            look_speed,
        }
    }

    pub fn move_in_plane_xz(&self, input: &Input, dt: f32, game_object: &mut GameObject) {
        let mut rotate = glam::Vec3::ZERO;

        if input.key_held(VirtualKeyCode::Right) {
            rotate[0] -= 1.0
        }
        if input.key_held(VirtualKeyCode::Left) {
            rotate[0] += 1.0
        }
        if input.key_held(VirtualKeyCode::Up) {
            rotate[1] += 1.0
        }
        if input.key_held(VirtualKeyCode::Down) {
            rotate[1] -= 1.0
        }

        if rotate.dot(rotate) > EPSILON {
            game_object.transform.rotation += self.look_speed * dt * rotate.normalize();
        }

        game_object.transform.rotation.y = game_object.transform.rotation.y.clamp(-1.5, 1.5);
        game_object.transform.rotation.x = game_object.transform.rotation.x % (2.0 * PI);

        let look_dir = glam::Vec3::new(
            game_object.transform.rotation.y.cos() * game_object.transform.rotation.x.sin(),
            game_object.transform.rotation.y.sin(),
            game_object.transform.rotation.y.cos() * game_object.transform.rotation.x.cos(),
        );

        let up = glam::Vec3::new(0.0, 1.0, 0.0);

        let dir = {
            let mut dir = look_dir;
            dir.y = 0.0;
            dir.normalize() * self.move_speed
        };

        let mut velocity = glam::Vec3::new(0.0, 0.0, 0.0);

        if input.key_held(VirtualKeyCode::W) {
            velocity -= dir;
        }

        if input.key_held(VirtualKeyCode::S) {
            velocity += dir;
        }

        if input.key_held(VirtualKeyCode::A) {
            velocity += dir.cross(up);
        }

        if input.key_held(VirtualKeyCode::D) {
            velocity -= dir.cross(up);
        }

        if input.key_held(VirtualKeyCode::E) {
            velocity.y -= self.move_speed;
        }

        if input.key_held(VirtualKeyCode::Q) {
            velocity.y += self.move_speed;
        }

        if velocity.dot(velocity) > EPSILON {
            game_object.transform.translation += self.move_speed * dt * velocity.normalize();
        }
    }
}
