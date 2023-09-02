use std::sync::{Arc, Mutex};

use ash::vk;
use assets::{Transform, ModelRegistry};
use glam::Vec3;

use crate::{
    renderer::Renderer,
    systems::{
        render::{RenderObject, Renderable},
        Systems,
    },
};

pub struct Grass {
    pub grass: RenderObject,
}

impl Grass {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        model_registry: &mut ModelRegistry,
        transform: Transform,
    ) -> Result<Arc<Mutex<Self>>, vk::Result> {
        let grass = RenderObject { model: model_registry.load("grass.glb"), transform }; 
        let grass = Arc::new(Mutex::new(Self { grass }));
        systems.render.add(grass.clone());
        Ok(grass)
    }
}

impl Renderable for Grass {
    fn get_objects(&self) -> Vec<RenderObject> {
        vec![self.grass.clone()]
    }
}
