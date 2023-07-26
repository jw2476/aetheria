use std::sync::{Arc, Mutex, Weak};

use ash::vk;
use assets::MeshRegistry;
use glam::{Quat, Vec3};
use rand::Rng;

use super::Sun;
use crate::{
    render::{Light, RenderObject, Renderable, RenderPass},
    renderer::Renderer,
    time::Time,
    transform::Transform,
};

const FIREFLY_SPEED: f32 = 60.0;

pub struct Firefly {
    light: Light,
    velocity: Vec3,
    origin: Vec3,
    front: RenderObject,
    back: RenderObject,
}

impl Firefly {
    pub fn new(
        renderer: &mut Renderer,
        render_pass: &mut RenderPass,
        mesh_registry: &mut MeshRegistry,
        position: Vec3,
        color: Vec3,
    ) -> Result<Arc<Mutex<Self>>, vk::Result> {
        let light = Light::new(position, 0.0, color);

        let front = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("firefly_front.obj")?
            .set_color(Vec3::new(0.0, 0.0, 0.0))
            .set_transform(Transform {
                translation: position,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            })
            .build()?;
        let back = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("firefly_back.obj")?
            .set_color(Vec3::new(1.0, 1.0, 0.0))
            .set_transform(Transform {
                translation: position,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            })
            .build()?;

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
            origin: position,
            front,
            back,
        }));


        render_pass.add_renderable(Arc::downgrade(&(firefly.clone() as Arc<Mutex<dyn Renderable>>)));
        Ok(firefly)
    }

    pub fn frame_finished(&mut self, sun: &Sun, time: &Time) {
        if sun.get_theta() > (std::f32::consts::PI / 3.0)
            && sun.get_theta() < (std::f32::consts::PI * (5.0 / 3.0))
        {
            self.light.strength = 300.0
                * ((sun.get_theta() / 2.0).sin() - sun.get_theta().cos())
                    .powf(1.5)
                    .min(1.0);
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
        self.front.transform.translation = self.light.position + Vec3::new(0.0, 5.0, 0.0);
        self.back.transform.translation = self.light.position + Vec3::new(0.0, 5.0, 0.0);

        let v = Vec3::new(self.velocity.x, 0.0, self.velocity.z).normalize();
        let rotation = Quat::from_rotation_arc(Vec3::new(0.0, 0.0, 1.0), v);
        self.front.transform.rotation = rotation.clone();
        self.back.transform.rotation = rotation.clone();
    }
}

impl AsRef<Light> for Firefly {
    fn as_ref(&self) -> &Light {
        &self.light
    }
}

impl Renderable for Firefly {
    fn get_objects(&self) -> Vec<RenderObject> {
        if self.light.strength != 0.0 {
            vec![self.front.clone(), self.back.clone()]
        } else {
            vec![]
        }
    }
}
