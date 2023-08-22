use crate::{
    data::Data,
    renderer::Renderer,
    systems::{
        interact::Interactable,
        render::{RenderObject, Renderable},
        Named, Positioned, Systems,
    },
    transform::Transform,
};
use ash::vk;
use assets::MeshRegistry;
use glam::Vec3;
use std::sync::{Arc, Mutex};

pub struct Furnace {
    render: RenderObject,
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
            .set_color(Vec3::new(0.5, 0.5, 0.5))
            .set_transform(transform)
            .build()?;

        let furnace = Arc::new(Mutex::new(Self { render }));
        systems.render.add(furnace.clone());
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
    fn interact(&mut self, _data: &mut Data) {
        println!("Yay")
    }

    fn active(&self) -> bool {
        true
    }
}
