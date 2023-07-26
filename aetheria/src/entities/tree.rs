use std::sync::{Arc, Mutex};

use ash::vk;
use assets::MeshRegistry;
use glam::Vec3;

use crate::{
    render::{RenderObject, Renderable, RenderPass},
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
        render_pass: &mut RenderPass,
        mesh_registry: &mut MeshRegistry,
        transform: Transform,
    ) -> Result<Arc<Mutex<Tree>>, vk::Result> {
        let trunk = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("tree.trunk.obj")?
            .set_color(Vec3::new(0.451, 0.243, 0.224))
            .set_transform(transform.clone())
            .build()?;
        let foliage = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("tree.foliage.obj")?
            .set_color(Vec3::new(0.984, 0.749, 0.141))
            .set_transform(transform.clone())
            .build()?;

        let tree = Arc::new(Mutex::new(Self {
            transform,
            trunk,
            foliage,
        }));

        render_pass.add_renderable(Arc::downgrade(&(tree.clone() as Arc<Mutex<dyn Renderable>>)));
        Ok(tree)
    }

    pub fn update_transform(&mut self) -> Result<(), vk::Result> {
        self.trunk.transform = self.transform.clone();
        self.foliage.transform = self.transform.clone();
        Ok(())
    }
}

impl Renderable for Tree {
    fn get_objects(&self) -> Vec<RenderObject> {
        vec![self.trunk.clone(), self.foliage.clone()]
    }
}
