use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use ash::vk;
use assets::{ModelRegistry, Transform};
use glam::Vec3;
use rand::Rng;

use crate::{entities::Firefly, renderer::Renderer, systems::Systems};

const NUM_FIREFLIES: u32 = 10;

pub struct Fireflies {
    fireflies: Vec<Arc<Mutex<Firefly>>>,
}

impl Fireflies {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        model_registry: &mut ModelRegistry,
    ) -> Result<Self, vk::Result> {
        let mut fireflies = Vec::new();

        let mut rng = rand::thread_rng();

        for _ in 0..NUM_FIREFLIES {
            let position = Vec3::new(
                rng.gen_range(-400.0..400.0),
                50.0,
                rng.gen_range(-400.0..400.0),
            );
            fireflies.push(
                Firefly::new(
                    renderer,
                    systems,
                    model_registry,
                    position,
                    Vec3::new(1.0, 1.0, 1.0),
                )
                .unwrap(),
            );
        }

        Ok(Self { fireflies })
    }
}

impl Deref for Fireflies {
    type Target = Vec<Arc<Mutex<Firefly>>>;

    fn deref(&self) -> &Self::Target {
        &self.fireflies
    }
}

impl DerefMut for Fireflies {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fireflies
    }
}
