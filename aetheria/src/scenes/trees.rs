use std::f32::consts::PI;

use ash::vk;
use assets::MeshRegistry;
use glam::{Quat, Vec3};
use rand::Rng;

use crate::{
    entities::Tree,
    renderer::{RenderObject, Renderable, Renderer},
    transform::Transform,
};

const NUM_TREES: u32 = 10;

pub struct Trees {
    trees: Vec<Tree>,
}

impl Trees {
    pub fn new(
        renderer: &mut Renderer,
        mesh_registry: &mut MeshRegistry,
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
            trees.push(Tree::new(renderer, mesh_registry, transform).unwrap());
        }

        Ok(Self { trees })
    }
}

impl Renderable for Trees {
    fn get_objects(&self) -> Vec<&RenderObject> {
        self.trees
            .iter()
            .flat_map(|tree| tree.get_objects())
            .collect::<Vec<&RenderObject>>()
    }
}
