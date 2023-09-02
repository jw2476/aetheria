use std::sync::{Arc, Mutex};

use ash::vk;
use assets::{Transform, ModelRegistry};
use glam::Vec3;

use crate::{
    data::{inventory::Inventory, Data},
    renderer::Renderer,
    systems::{
        interact::Interactable,
        render::{RenderObject, Renderable},
        Named, Positioned, Systems,
    },
};
use common::item::{Item, ItemStack};

pub struct Tree {
    pub tree: RenderObject,
    gathered: bool,
}

impl Tree {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        model_registry: &mut ModelRegistry,
        transform: Transform,
    ) -> Result<Arc<Mutex<Tree>>, vk::Result> {
        let tree = RenderObject { model: model_registry.load("tree.glb"), transform };

        let tree = Arc::new(Mutex::new(Self {
            tree,
            gathered: false,
        }));

        systems.render.add(tree.clone());
        systems.interact.add(tree.clone());

        Ok(tree)
    }
}

impl Renderable for Tree {
    fn get_objects(&self) -> Vec<RenderObject> {
        if !self.gathered {
            vec![self.tree.clone()]
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
        self.tree.transform.translation
    }
}

impl Interactable for Tree {
    fn interact(&mut self, data: &mut Data) {
        data.inventory.add(ItemStack {
            item: Item::Wood,
            amount: 1,
        });
        self.gathered = true;
    }

    fn active(&self) -> bool {
        !self.gathered
    }
}
