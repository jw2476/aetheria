use std::{f32::consts::PI, ops::Deref, time::SystemTime, sync::{Arc, Mutex}};

use glam::{Quat, Vec3};

use crate::{render::Light, time::Time};

pub struct Sun {
    noon_pos: Vec3,
    pub light: Light,
    theta: f32,
}

impl Sun {
    pub fn new(noon_pos: Vec3, color: Vec3) -> Arc<Mutex<Self>> {
        let seconds = SystemTime::UNIX_EPOCH.elapsed().unwrap().as_secs();
        let mut sun = Self {
            noon_pos,
            light: Light::new(noon_pos, 0.0, color),
            theta: (seconds % 120) as f32 * (PI / 60.0),
        };
        sun.update_theta(sun.theta);
        Arc::new(Mutex::new(sun))
    }

    pub fn update_theta(&mut self, theta: f32) {
        self.theta = theta % (std::f32::consts::PI * 2.0);
        self.light.position =
            Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), self.theta) * self.noon_pos;
        self.light.color = Vec3::new(1.0, 1.0, 1.0);
        self.light.strength =
            self.light.position.length().powf(2.0) * 0.5 * self.theta.cos().powf(0.13).max(0.0);
        self.light.strength = self.light.strength.max(0.0);
    }

    pub fn frame_finished(&mut self, time: &Time) {
        self.update_theta(self.theta + (time.delta_seconds() * (PI / 60.0)));
    }

    pub fn get_theta(&self) -> f32 {
        self.theta
    }
}

impl Deref for Sun {
    type Target = Light;

    fn deref(&self) -> &Self::Target {
        &self.light
    }
}
