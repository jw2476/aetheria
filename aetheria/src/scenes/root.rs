use std::{net::UdpSocket, ops::Deref};

use ash::vk;
use assets::MeshRegistry;
use glam::{Quat, Vec2, Vec3};

use crate::{
    camera::Camera,
    entities::{Grass, Player, Sun},
    input::{Keyboard, Mouse},
    renderer::Renderer,
    render::{Light, Renderable, RenderObject},
    time::Time,
    transform::Transform,
};

use super::{Fireflies, Trees};

pub struct RootScene {
    pub player: Player,
    pub sun: Sun,
    pub grass: Grass,
    pub trees: Trees,
    pub fireflies: Fireflies,
}

impl RootScene {
    pub fn new(
        renderer: &mut Renderer,
        mesh_registry: &mut MeshRegistry,
    ) -> Result<Self, vk::Result> {
        let mut player = {
            let transform = Transform {
                translation: Vec3::new(0.0, 10.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            };
            Player::new(renderer, mesh_registry, transform).unwrap()
        };
        let sun = Sun::new(Vec3::new(0.0, 1000000.0, 0.0), Vec3::new(0.8, 1.0, 0.5));
        let grass = Grass::new(renderer, mesh_registry, Transform::IDENTITY).unwrap();

        let trees = Trees::new(renderer, mesh_registry)?;
        let fireflies = Fireflies::new(renderer, mesh_registry)?;

        Ok(Self {
            player,
            sun,
            grass,
            trees,
            fireflies,
        })
    }

    pub fn get_lights(&self) -> Vec<Light> {
        let mut data = self
            .fireflies
            .iter()
            .map(|firefly| *firefly.as_ref())
            .collect::<Vec<Light>>();
        data.push(self.sun.light);
        data.push(self.player.light);
        data
    }

    pub fn frame_finished(
        &mut self,
        keyboard: &Keyboard,
        mouse: &Mouse,
        camera: &Camera,
        time: &Time,
        viewport: Vec2,
        socket: &UdpSocket,
    ) {
        self.player
            .frame_finished(keyboard, mouse, camera, time, viewport, socket);
        self.sun.frame_finished(time);
        self.fireflies
            .iter_mut()
            .for_each(|firefly| firefly.frame_finished(&self.sun, time));
    }
}

impl Renderable for RootScene {
    fn get_objects(&self) -> Vec<&RenderObject> {
        vec![
            self.player.get_objects(),
            self.grass.get_objects(),
            self.trees.get_objects(),
            self.fireflies.get_objects(),
        ]
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<&RenderObject>>()
    }
}
