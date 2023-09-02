use crate::{
    data::{Data, Recipe},
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

pub struct CraftingBench {
    render: RenderObject,
}

impl CraftingBench {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        model_registry: &mut ModelRegistry,
        transform: Transform,
    ) -> Result<Arc<Mutex<Self>>, vk::Result> {
        let render = RenderObject { model: model_registry.load("crafting_bench.glb"), transform };

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
        self.render.transform.translation
    }
}

impl Interactable for CraftingBench {
    fn active(&self) -> bool {
        true
    }

    fn interact(&mut self, data: &mut Data) {
        data.recipe_selections = Some(vec![
            Recipe {
                ingredients: vec![
                    ItemStack {
                        item: Item::Wood,
                        amount: 3,
                    },
                    ItemStack {
                        item: Item::Fireglow,
                        amount: 2,
                    },
                ],
                outputs: vec![ItemStack {
                    item: Item::Lamp,
                    amount: 1,
                }],
            },
            Recipe {
                ingredients: vec![
                    ItemStack {
                        item: Item::Wood,
                        amount: 2,
                    },
                    ItemStack {
                        item: Item::CopperIngot,
                        amount: 2,
                    },
                ],
                outputs: vec![ItemStack {
                    item: Item::CopperSword,
                    amount: 1,
                }],
            },
        ])
    }
}
