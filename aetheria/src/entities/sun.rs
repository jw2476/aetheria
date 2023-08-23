use std::{
    f32::consts::PI,
    ops::Deref,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use glam::{Quat, Vec3};

use crate::{
    systems::{render::{Emissive, Light}, Systems},
    time::Time, data::Data,
};

pub struct Sun {
    noon_pos: Vec3,
    pub light: Light,
    theta: f32,
}

impl Sun {
    pub fn new(systems: &mut Systems, noon_pos: Vec3, color: Vec3) -> Arc<Mutex<Self>> {
        let seconds = SystemTime::UNIX_EPOCH.elapsed().unwrap().as_secs();
        let mut sun = Self {
            noon_pos,
            light: Light::new(noon_pos, 0.0, color),
            theta: (seconds % 120) as f32 * (PI / 60.0),
        };
        sun.update_theta(sun.theta);

        let sun = Arc::new(Mutex::new(sun));

        systems.render.add_light(sun.clone());

        sun
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

impl Emissive for Sun {
    fn get_lights(&self, _: &Data) -> Vec<Light> {
        vec![self.light]
    }
}
