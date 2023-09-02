use std::{
    f32::consts::PI,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use crate::{entities::CopperOre, renderer::Renderer, systems::Systems};
use ash::vk;
use assets::{Transform, ModelRegistry};
use glam::{Quat, Vec3};
use rand::Rng;

const NUM_ORES: u32 = 10;

pub struct Ores {
    trees: Vec<Arc<Mutex<CopperOre>>>,
}

impl Ores {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        model_registry: &mut ModelRegistry,
    ) -> Result<Self, vk::Result> {
        let mut trees = Vec::new();

        let mut rng = rand::thread_rng();

        for _ in 0..NUM_ORES {
            let translation = Vec3::new(
                rng.gen_range(-400.0..400.0),
                0.0,
                rng.gen_range(-400.0..400.0),
            );
            let rotation = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), rng.gen_range(-PI..PI));
            let transform = Transform {
                translation,
                rotation,
                scale: Vec3::new(0.1, 0.1, 0.1),
            };
            trees.push(CopperOre::new(renderer, systems, model_registry, transform).unwrap());
        }

        Ok(Self { trees })
    }
}

impl Deref for Ores {
    type Target = Vec<Arc<Mutex<CopperOre>>>;

    fn deref(&self) -> &Self::Target {
        &self.trees
    }
}

impl DerefMut for Ores {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.trees
    }
}
