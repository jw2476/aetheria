use std::{
    f32::consts::PI,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use ash::vk;
use assets::{ModelRegistry, Transform};
use glam::{Quat, Vec3};
use rand::Rng;

use crate::{entities::Tree, renderer::Renderer, systems::Systems};

const NUM_TREES: u32 = 10;

pub struct Trees {
    trees: Vec<Arc<Mutex<Tree>>>,
}

impl Trees {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        model_registry: &mut ModelRegistry,
    ) -> Result<Self, vk::Result> {
        let mut trees = Vec::new();

        let mut rng = rand::thread_rng();

        for _ in 0..NUM_TREES {
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
            trees.push(Tree::new(renderer, systems, model_registry, transform).unwrap());
        }

        Ok(Self { trees })
    }
}

impl Deref for Trees {
    type Target = Vec<Arc<Mutex<Tree>>>;

    fn deref(&self) -> &Self::Target {
        &self.trees
    }
}

impl DerefMut for Trees {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.trees
    }
}
