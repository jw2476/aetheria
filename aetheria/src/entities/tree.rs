use ash::vk;
use assets::MeshRegistry;
use glam::Vec3;

use crate::{
    render::{RenderObject, Renderable},
    renderer::Renderer,
    transform::Transform,
};

pub struct Tree {
    pub transform: Transform,
    trunk: RenderObject,
    foliage: RenderObject,
}

impl Tree {
    pub fn new(
        renderer: &mut Renderer,
        mesh_registry: &mut MeshRegistry,
        transform: Transform,
    ) -> Result<Tree, vk::Result> {
        let trunk = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("tree.trunk.obj")?
            .set_color(Vec3::new(0.451, 0.243, 0.224))
            .set_transform(transform.clone())
            .build()?;
        let foliage = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("tree.foliage.obj")?
            .set_color(Vec3::new(0.388, 0.780, 0.302))
            .set_transform(transform.clone())
            .build()?;

        Ok(Self {
            transform,
            trunk,
            foliage,
        })
    }

    pub fn update_transform(&mut self) -> Result<(), vk::Result> {
        self.trunk.transform = self.transform.clone();
        self.foliage.transform = self.transform.clone();
        Ok(())
    }
}

impl Renderable for Tree {
    fn get_objects(&self) -> Vec<&RenderObject> {
        vec![&self.trunk, &self.foliage]
    }
}
