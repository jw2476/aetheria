use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

use ash::vk;
use assets::MeshRegistry;
use glam::{Quat, Vec2, Vec3};

use crate::{
    camera::Camera,
    entities::{Furnace, Grass, Player, Sun},
    input::{Keyboard, Mouse},
    renderer::Renderer,
    socket::Socket,
    systems::{render::Light, Systems},
    time::Time,
    transform::Transform,
};

use super::{Fireflies, Trees};

pub struct RootScene {
    pub player: Arc<Mutex<Player>>,
    pub sun: Arc<Mutex<Sun>>,
    pub grass: Arc<Mutex<Grass>>,
    pub trees: Trees,
    pub fireflies: Fireflies,
    pub furnace: Arc<Mutex<Furnace>>,
}

impl RootScene {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        mesh_registry: &mut MeshRegistry,
    ) -> Result<Self, vk::Result> {
        let player = {
            let transform = Transform {
                translation: Vec3::new(0.0, 10.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            };
            Player::new(renderer, systems, mesh_registry, transform).unwrap()
        };
        let sun = Sun::new(systems, Vec3::new(0.0, 1000000.0, 0.0), Vec3::new(0.8, 1.0, 0.5));
        let grass = Grass::new(renderer, systems, mesh_registry, Transform::IDENTITY).unwrap();

        let trees = Trees::new(renderer, systems, mesh_registry)?;
        let fireflies = Fireflies::new(renderer, systems, mesh_registry)?;

        let furnace = Furnace::new(
            renderer,
            systems,
            mesh_registry,
            Transform {
                translation: Vec3::new(100.0, 0.0, 100.0),
                scale: Vec3::new(0.2, 0.2, 0.2),
                ..Default::default()
            },
        )?;

        Ok(Self {
            player,
            sun,
            grass,
            trees,
            fireflies,
            furnace,
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
