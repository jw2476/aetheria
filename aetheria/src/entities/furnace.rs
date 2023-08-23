use crate::{
    data::{Recipe, Data},
    renderer::Renderer,
    systems::{
        interact::Interactable,
        render::{RenderObject, Renderable, Light, Emissive},
        Named, Positioned, Systems,
    },
    transform::Transform,
};
use ash::vk;
use assets::MeshRegistry;
use common::item::{Item,ItemStack};
use glam::Vec3;
use std::sync::{Arc, Mutex};

pub struct Furnace {
    render: RenderObject,
    light: Light
}

impl Furnace {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        mesh_registry: &mut MeshRegistry,
        transform: Transform,
    ) -> Result<Arc<Mutex<Self>>, vk::Result> {
        let render = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("furnace.obj")?
            .set_color(Vec3::new(0.7, 0.5, 0.5))
            .set_transform(transform.clone())
            .build()?;

        let light = Light::new(transform.translation + Vec3::new(0.0, 20.0, -10.0), 4000.0, Vec3::new(0.976, 0.451, 0.086));

        let furnace = Arc::new(Mutex::new(Self { render, light }));
        systems.render.add(furnace.clone());
        systems.render.add_light(furnace.clone());
        systems.interact.add(furnace.clone());

        Ok(furnace)
    }
}

impl Renderable for Furnace {
    fn get_objects(&self) -> Vec<RenderObject> {
        vec![self.render.clone()]
    }
}

impl Named for Furnace {
    fn get_name(&self) -> String {
        "Furnace".to_owned()
    }
}

impl Positioned for Furnace {
    fn get_position(&self) -> Vec3 {
        self.render.transform.translation
    }
}

impl Interactable for Furnace {
    fn interact(&mut self, data: &mut Data) {
        data.current_recipe = Some(Recipe {
            ingredients: vec![ItemStack { item: Item::CopperOre, amount: 3 }],
            outputs: vec![ItemStack { item: Item::CopperIngot, amount: 1 }]
        })
    }

    fn active(&self) -> bool {
        true
    }
}

impl Emissive for Furnace {
    fn get_lights(&self, _: &Data) -> Vec<Light> {
        vec![self.light]
    }
}
