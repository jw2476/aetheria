use std::sync::{Arc, Mutex};

use ash::vk;
use assets::MeshRegistry;
use glam::Vec3;

use crate::{
    renderer::Renderer,
    systems::{
        gather::Gatherable,
        render::{RenderObject, Renderable},
        Named, Positioned, Systems,
    },
    transform::Transform,
};
use common::item::{Inventory, Item, ItemStack};

pub struct Tree {
    pub transform: Transform,
    trunk: RenderObject,
    foliage: RenderObject,
    gathered: bool,
}

impl Tree {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
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
            gathered: false,
        }));

        systems
            .render
            .add_renderable(tree.clone() as Arc<Mutex<dyn Renderable>>);
        systems.gather.add_gatherable(tree.clone());

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
        if !self.gathered {
            vec![self.trunk.clone(), self.foliage.clone()]
        } else {
            Vec::new()
        }
    }
}

impl Named for Tree {
    fn get_name(&self) -> String {
        "Tree".to_owned()
    }
}

impl Positioned for Tree {
    fn get_position(&self) -> Vec3 {
        self.transform.translation
    }
}

impl Gatherable for Tree {
    fn gather(&mut self, inventory: &mut Inventory) {
        inventory.add(ItemStack {
            item: Item::Wood,
            amount: 1,
        });
        self.gathered = true;
    }

    fn is_gatherable(&self) -> bool {
        !self.gathered
    }
}
