use ash::vk;
use assets::MeshRegistry;
use glam::Vec3;

use crate::{
    render::{RenderObject, Renderable},
    renderer::Renderer,
    transform::Transform,
};

pub struct Grass {
    pub transform: Transform,
    grass: RenderObject,
}

impl Grass {
    pub fn new(
        renderer: &mut Renderer,
        mesh_registry: &mut MeshRegistry,
        transform: Transform,
    ) -> Result<Self, vk::Result> {
        let grass = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("grass.obj")?
            .set_color(Vec3::new(0.388, 0.780, 0.302))
            .set_transform(transform.clone())
            .build()?;
        Ok(Self { transform, grass })
    }
}

impl Renderable for Grass {
    fn get_objects(&self) -> Vec<&RenderObject> {
        vec![&self.grass]
    }
}
