use std::f32::EPSILON;

use ash::vk;
use bytemuck::cast_slice;
use glam::{Mat4, Vec3, Quat};

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
        let target = Vec3::new(0.0, 0.5, 0.0);

        let mut camera = Self {
            theta,
            actual_theta: theta,
            target,
            actual_target: target,
            width,
            height
        };

        camera.update();

        Ok(camera)
    }

    pub fn update(&mut self) {
        let aspect = self.width / self.height;
        let eye = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), self.actual_theta) * Vec3::new(0.0, 5.0 * 35.264_f32.tan(), 5.0);
        let view = Mat4::look_at_rh(eye + self.actual_target, self.actual_target, Vec3::new(0.0, 1.0, 0.0));
        let mut proj = Mat4::orthographic_rh(-3.0 * aspect, 3.0 * aspect, -3.0, 3.0, 0.1, 100.0);
        //let mut proj = Mat4::perspective_rh(45.0_f32.to_radians(), aspect, 0.1, 100.0);

        proj.col_mut(1)[1] *= -1.0;

        /*let vp = [view.to_cols_array(), proj.to_cols_array()]
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<f32>>();
        let vp = cast_slice::<f32, u8>(&vp);*/
    }

    pub fn frame_finished(&mut self) {
        if (self.actual_theta - self.theta).abs() > EPSILON {
            self.actual_theta += (self.theta - self.actual_theta) * Self::DAMPING;
        }

        if (self.actual_target - self.target).length() > EPSILON {
            self.actual_target += (self.target - self.actual_target) * Self::DAMPING;
        }

        self.update();
    }

    pub fn get_rotation(&self) -> Quat {
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), self.theta)
    }
}
