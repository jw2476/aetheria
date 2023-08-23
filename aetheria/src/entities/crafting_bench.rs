use crate::{systems::{render::{RenderObject, Renderable}, Systems, Named, Positioned, interact::Interactable}, renderer::Renderer, transform::Transform, data::{Data, Recipe}};
use ash::vk;
use assets::MeshRegistry;
use common::item::{ItemStack, Item};
use std::sync::{Arc, Mutex};
use glam::Vec3;

pub struct CraftingBench {
    render: RenderObject
}

impl CraftingBench {
    pub fn new(renderer: &mut Renderer, systems: &mut Systems, mesh_registry: &mut MeshRegistry, transform: Transform) -> Result<Arc<Mutex<Self>>, vk::Result> {
        let render = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("crafting_bench.obj")?
            .set_color(Vec3::new(0.486, 0.176, 0.071))
            .set_transform(transform)
            .build()?;

        let bench = Arc::new(Mutex::new(Self { render }));

        systems.render.add(bench.clone());
        systems.interact.add(bench.clone());

        Ok(bench)
    }
}

impl Renderable for CraftingBench {
    fn get_objects(&self) -> Vec<RenderObject> {
        vec![self.render.clone()]
    }
}

impl Named for CraftingBench {
    fn get_name(&self) -> String {
        "Crafting Bench".to_owned()
    }
}

impl Positioned for CraftingBench {
    fn get_position(&self) -> Vec3 {
        self.render.transform.translation.clone()
    }
}

impl Interactable for CraftingBench {
    fn active(&self) -> bool {
        true
    }

    fn interact(&mut self, data: &mut Data) {
         data.recipe_selections = Some(vec![
            Recipe {
                ingredients: vec![ItemStack { item: Item::Wood, amount: 3 }, ItemStack { item: Item::Fireglow, amount: 2 }],
                outputs: vec![ItemStack { item: Item::Lamp, amount: 1 }]
            },
            Recipe {
                ingredients: vec![ItemStack { item: Item::Wood, amount: 2 }, ItemStack { item: Item::CopperIngot, amount: 2 }],
                outputs: vec![ItemStack { item: Item::CopperSword, amount: 1 }]
            }
         ]) 
    }
}
