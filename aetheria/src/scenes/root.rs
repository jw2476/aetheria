use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

use ash::vk;
use assets::{Transform, ModelRegistry};
use glam::{Quat, Vec2, Vec3};

use crate::{
    camera::Camera,
    entities::{CraftingBench, Furnace, Grass, Player, Sun},
    input::{Keyboard, Mouse},
    renderer::Renderer,
    socket::Socket,
    systems::{render::Light, Systems},
    time::Time,
};

use super::{Fireflies, Ores, Trees};

pub struct RootScene {
    pub player: Arc<Mutex<Player>>,
    pub sun: Arc<Mutex<Sun>>,
    pub grass: Arc<Mutex<Grass>>,
    pub trees: Trees,
    pub fireflies: Fireflies,
    pub furnace: Arc<Mutex<Furnace>>,
    pub crafting_bench: Arc<Mutex<CraftingBench>>,
    pub ores: Ores,
}

impl RootScene {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        model_registry: &mut ModelRegistry,
    ) -> Result<Self, vk::Result> {
        let player = {
            let transform = Transform {
                translation: Vec3::new(0.0, 10.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            };
            Player::new(renderer, systems, model_registry, transform).unwrap()
        };
        let sun = Sun::new(
            systems,
            Vec3::new(0.0, 1000000.0, 0.0),
            Vec3::new(0.8, 1.0, 0.5),
        );
        let grass = Grass::new(renderer, systems, model_registry, Transform::IDENTITY).unwrap();

        let trees = Trees::new(renderer, systems, model_registry)?;
        let fireflies = Fireflies::new(renderer, systems, model_registry)?;

        let furnace = Furnace::new(
            renderer,
            systems,
            model_registry,
            Transform {
                translation: Vec3::new(100.0, 0.0, 100.0),
                scale: Vec3::new(0.2, 0.2, 0.2),
                ..Default::default()
            },
        )?;

        let ores = Ores::new(renderer, systems, model_registry)?;

        let crafting_bench = CraftingBench::new(
            renderer,
            systems,
            model_registry,
            Transform {
                translation: Vec3::new(100.0, 0.0, 30.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.1, 0.1, 0.1),
            },
        )?;
        Ok(Self {
            player,
            sun,
            grass,
            trees,
            fireflies,
            furnace,
            crafting_bench,
            ores,
        })
    }

    pub fn frame_finished(
        &mut self,
        keyboard: &Keyboard,
        mouse: &Mouse,
        camera: &Camera,
        time: &Time,
        viewport: Vec2,
        socket: &Socket,
    ) {
        self.player
            .lock()
            .unwrap()
            .frame_finished(keyboard, mouse, camera, time, viewport, socket);
        self.sun.lock().unwrap().frame_finished(time);
        self.fireflies.iter_mut().for_each(|firefly| {
            firefly
                .lock()
                .unwrap()
                .frame_finished(&self.sun.lock().unwrap(), time)
        });
    }
}
