use std::sync::{Arc, Mutex, Weak};

use ash::vk;
use assets::{ModelRegistry, Transform};
use glam::{Quat, Vec3};
use rand::Rng;

use super::Sun;
use crate::{
    data::{inventory::Inventory, Data},
    renderer::Renderer,
    systems::{
        interact::Interactable,
        render::{Emissive, Light, RenderObject, Renderable, System},
        Named, Positioned, Systems,
    },
    time::Time,
};
use common::item::{Item, ItemStack};

const FIREFLY_SPEED: f32 = 60.0;

pub struct Firefly {
    light: Light,
    velocity: Vec3,
    origin: Vec3,
    render: RenderObject,
    gathered: bool,
}

impl Firefly {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        model_registry: &mut ModelRegistry,
        translation: Vec3,
        color: Vec3,
    ) -> Result<Arc<Mutex<Self>>, vk::Result> {
        let light = Light::new(translation, 0.0, color);

        let transform = Transform {
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let render = RenderObject {
            model: model_registry.load("firefly.glb"),
            transform,
        };

        let mut rng = rand::thread_rng();
        let velocity = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        )
        .normalize_or_zero();
        let firefly = Arc::new(Mutex::new(Self {
            light,
            velocity,
            origin: translation,
            render,
            gathered: false,
        }));

        systems.render.add(firefly.clone());
        systems.render.add_light(firefly.clone());
        systems.interact.add(firefly.clone());
        Ok(firefly)
    }

    pub fn frame_finished(&mut self, sun: &Sun, time: &Time) {
        if sun.get_theta() > (std::f32::consts::PI / 3.0)
            && sun.get_theta() < (std::f32::consts::PI * (5.0 / 3.0))
        {
            self.light.strength = 300.0
                * ((sun.get_theta() / 2.0).sin() - sun.get_theta().cos())
                    .powf(1.5)
                    .min(1.0)
                * !self.gathered as u32 as f32;
        } else {
            self.light.strength = 0.0
        }

        self.light.position += self.velocity * FIREFLY_SPEED * time.delta_seconds();

        let mut rng = rand::thread_rng();
        let random_vec3 = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        )
        .normalize_or_zero();
        let origin_direction = (self.origin - self.light.position).normalize_or_zero();
        let origin_bias = ((self.origin - self.light.position).length() - 100.0) / 100.0;
        self.velocity = (self.velocity + random_vec3 * 0.1 + origin_direction * origin_bias)
            .normalize_or_zero();

        self.light.position.y = self.light.position.y.clamp(5.0, 15.0);
        self.render.transform.translation = self.light.position + Vec3::new(0.0, 5.0, 0.0);

        let v = Vec3::new(self.velocity.x, 0.0, self.velocity.z).normalize();
        let rotation = Quat::from_rotation_arc(Vec3::new(0.0, 0.0, 1.0), v);
        self.render.transform.rotation = rotation;
    }
}

impl Emissive for Firefly {
    fn get_lights(&self, _: &Data) -> Vec<Light> {
        vec![self.light]
    }
}

impl Renderable for Firefly {
    fn get_objects(&self) -> Vec<RenderObject> {
        if self.light.strength != 0.0 && !self.gathered {
            vec![self.render.clone()]
        } else {
            vec![]
        }
    }
}

impl Named for Firefly {
    fn get_name(&self) -> String {
        "Sunset Firefly".to_owned()
    }
}

impl Positioned for Firefly {
    fn get_position(&self) -> Vec3 {
        self.light.position
    }
}

impl Interactable for Firefly {
    fn interact(&mut self, data: &mut crate::data::Data) {
        data.inventory.add(ItemStack {
            item: Item::Fireglow,
            amount: 1,
        });
        self.gathered = true;
    }

    fn active(&self) -> bool {
        !self.gathered && self.light.strength > 0.0
    }
}
