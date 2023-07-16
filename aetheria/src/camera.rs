use std::f32::EPSILON;

use ash::vk;
use bytemuck::{cast_slice, cast_slice_mut};
use glam::{Mat4, Quat, Vec3};
use vulkan::Buffer;

use crate::renderer::Renderer;

pub struct Camera {
    pub target: Vec3,
    actual_target: Vec3,
    pub theta: f32,
    actual_theta: f32,

    pub width: f32,
    pub height: f32,

    pub buffer: Buffer
}

impl Camera {
    const DAMPING: f32 = 0.2;

    pub fn new(width: f32, height: f32, renderer: &Renderer) -> Result<Self, vk::Result> {
        let theta = -45.01_f32.to_radians();
        let target = Vec3::new(0.0, 0.0, 0.0);

        let camera = Self {
            theta,
            actual_theta: theta,
            target,
            actual_target: target,
            width,
            height,
            buffer: Buffer::new(&renderer, [0_u8; 32], vk::BufferUsageFlags::UNIFORM_BUFFER)?
        };

        Ok(camera)
    }

    fn pad_vec3(data: Vec3) -> [f32; 4] {
        [data.x, data.y, data.z, 0.0]
    }
    
    pub fn update_buffer(&mut self) {
        let mut eye = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), self.actual_theta)
            * Vec3::new(0.0, 500.0 * 2.0_f32.powf(-0.5), 500.0);
        eye += self.actual_target;

        let vp = [Self::pad_vec3(eye), Self::pad_vec3(self.actual_target)]
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<f32>>();
        let vp = cast_slice::<f32, u8>(&vp);
        self.buffer.upload(vp);
    }

    pub fn frame_finished(&mut self) {
        if (self.actual_theta - self.theta).abs() > EPSILON {
            self.actual_theta += (self.theta - self.actual_theta) * Self::DAMPING;
        }

        if (self.actual_target - self.target).length() > EPSILON {
            self.actual_target += (self.target - self.actual_target) * Self::DAMPING;
        }
        
        self.update_buffer();
    }

    pub fn get_rotation(&self) -> Quat {
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), self.theta)
    }
}
