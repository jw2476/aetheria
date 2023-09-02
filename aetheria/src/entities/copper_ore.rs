use crate::{
    data::Data,
    renderer::Renderer,
    systems::{
        interact::Interactable,
        render::{RenderObject, Renderable},
        Named, Positioned, Systems,
    },
};
use ash::vk;
use assets::{Transform, ModelRegistry};
use common::item::{Item, ItemStack};
use glam::Vec3;
use std::sync::{Arc, Mutex};

pub struct CopperOre {
    render: RenderObject,
    gathered: bool,
}

impl CopperOre {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        model_registry: &mut ModelRegistry,
        transform: Transform,
    ) -> Result<Arc<Mutex<Self>>, vk::Result> {
        let render = RenderObject { model: model_registry.load("copper_ore.glb"), transform };

        let ore = Arc::new(Mutex::new(Self {
            render,
            gathered: false,
        }));

        systems.render.add(ore.clone());
        systems.interact.add(ore.clone());

        Ok(ore)
    }
}

impl Renderable for CopperOre {
    fn get_objects(&self) -> Vec<RenderObject> {
        if self.gathered {
            Vec::new()
        } else {
            vec![self.render.clone()]
        }
    }
}

impl Named for CopperOre {
    fn get_name(&self) -> String {
        "Copper Ore".to_owned()
    }
}

impl Positioned for CopperOre {
    fn get_position(&self) -> Vec3 {
        self.render.transform.translation.clone()
    }
}

impl Interactable for CopperOre {
    fn active(&self) -> bool {
        !self.gathered
    }

    fn interact(&mut self, data: &mut Data) {
        data.inventory.add(ItemStack {
            item: Item::CopperOre,
            amount: 1,
        });
        self.gathered = true;
    }
}
