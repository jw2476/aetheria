use std::ops::{Deref, DerefMut};

use ash::vk;
use assets::MeshRegistry;
use glam::Vec3;
use rand::Rng;

use crate::{entities::Firefly, renderer::Renderer};

const NUM_FIREFLIES: u32 = 10;

pub struct Fireflies {
    fireflies: Vec<Firefly>,
}

impl Fireflies {
    pub fn new(
        renderer: &mut Renderer,
        mesh_registry: &mut MeshRegistry,
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
                Firefly::new(renderer, mesh_registry, position, Vec3::new(1.0, 1.0, 1.0)).unwrap(),
            );
        }

        Ok(Self { fireflies })
    }
}

impl Deref for Fireflies {
    type Target = Vec<Firefly>;

    fn deref(&self) -> &Self::Target {
        &self.fireflies
    }
}

impl DerefMut for Fireflies {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fireflies
    }
}
