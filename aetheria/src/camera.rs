use std::f32::EPSILON;

use ash::vk;
use bytemuck::cast_slice;
use glam::{Mat4, Vec3, Quat};
use vulkan::Buffer;

pub struct Camera {
    pub target: Vec3,
    actual_target: Vec3,
    pub theta: f32,
    actual_theta: f32,

    pub width: f32,
    pub height: f32,
}

impl Camera {
    const DAMPING: f32 = 0.2;

    pub fn new(width: f32, height: f32) -> Result<Self, vk::Result> {
        let theta = 0.0;
        let target = Vec3::new(0.0, 0.0, 0.0);

        let camera = Self {
            theta,
            actual_theta: theta,
            target,
            actual_target: target,
            width,
            height
        };

        Ok(camera)
    }

    pub fn update_buffer(&self, buffer: &mut Buffer) {
        let mut eye = Vec3::new(0.0, 0.0, 0.0);
        eye += self.actual_target;

        let vp = [eye.to_array(), self.actual_target.to_array()]
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<f32>>();
        let vp = cast_slice::<f32, u8>(&vp);
        buffer.upload(vp);
    }

    pub fn frame_finished(&mut self) {
        if (self.actual_theta - self.theta).abs() > EPSILON {
            self.actual_theta += (self.theta - self.actual_theta) * Self::DAMPING;
        }

        if (self.actual_target - self.target).length() > EPSILON {
            self.actual_target += (self.target - self.actual_target) * Self::DAMPING;
        }
    }

    pub fn get_rotation(&self) -> Quat {
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), self.theta)
    }
}
