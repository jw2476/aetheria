use std::sync::{Arc, Mutex};

use ash::vk;
use assets::MeshRegistry;
use glam::Vec3;

use crate::{
    render::{RenderObject, RenderPass, Renderable},
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
        render_pass: &mut RenderPass,
        mesh_registry: &mut MeshRegistry,
        transform: Transform,
    ) -> Result<Arc<Mutex<Self>>, vk::Result> {
        let grass = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("grass.obj")?
            .set_color(Vec3::new(0.290, 0.871, 0.502))
            .set_transform(transform.clone())
            .build()?;
        let grass = Arc::new(Mutex::new(Self { transform, grass }));
        render_pass.add_renderable(Arc::downgrade(
            &(grass.clone() as Arc<Mutex<dyn Renderable>>),
        ));
        Ok(grass)
    }
}

impl Renderable for Grass {
    fn get_objects(&self) -> Vec<RenderObject> {
        vec![self.grass.clone()]
    }
}
