use ash::vk;
use assets::MeshRegistry;
use common::item::{ItemStack, Item};
use glam::Vec3;
use crate::{systems::{render::{RenderObject, Renderable}, Systems, Named, Positioned, interact::Interactable}, renderer::Renderer, data::Data, transform::Transform};
use std::sync::{Arc, Mutex};

pub struct CopperOre {
    rock: RenderObject,
    metal: RenderObject,
    gathered: bool
}

impl CopperOre {
    pub fn new(renderer: &mut Renderer, systems: &mut Systems, mesh_registry: &mut MeshRegistry, transform: Transform) -> Result<Arc<Mutex<Self>>, vk::Result> {
        let rock = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("copper_ore.rock.obj")?
            .set_color(Vec3::new(0.322, 0.322, 0.322))
            .set_transform(transform.clone())
            .build()?;

        let metal = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("copper_ore.metal.obj")?
            .set_color(Vec3::new(0.722, 0.451, 0.200))
            .set_transform(transform)
            .build()?;

        let ore = Arc::new(Mutex::new(Self { rock, metal, gathered: false }));

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
            vec![self.rock.clone(), self.metal.clone()]
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
        self.rock.transform.translation.clone()
    }
}

impl Interactable for CopperOre {
    fn active(&self) -> bool {
       !self.gathered 
    }

    fn interact(&mut self, data: &mut Data) {
        data.inventory.add(ItemStack { item: Item::CopperOre, amount: 1 });
        self.gathered = true;
    }
}
