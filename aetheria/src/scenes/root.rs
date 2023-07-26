use std::{net::UdpSocket, ops::Deref, sync::{Arc, Mutex}};

use ash::vk;
use assets::MeshRegistry;
use glam::{Quat, Vec2, Vec3};

use crate::{
    camera::Camera,
    entities::{Grass, Player, Sun},
    input::{Keyboard, Mouse},
    render::{Light, RenderObject, Renderable, RenderPass},
    renderer::Renderer,
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
}

impl RootScene {
    pub fn new(
        renderer: &mut Renderer,
        render_pass: &mut RenderPass,
        mesh_registry: &mut MeshRegistry,
    ) -> Result<Self, vk::Result> {
        let mut player = {
            let transform = Transform {
                translation: Vec3::new(0.0, 10.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            };
            Player::new(renderer, render_pass, mesh_registry, transform).unwrap()
        };
        let sun = Sun::new(Vec3::new(0.0, 1000000.0, 0.0), Vec3::new(0.8, 1.0, 0.5));
        let grass = Grass::new(renderer, render_pass, mesh_registry, Transform::IDENTITY).unwrap();

        let trees = Trees::new(renderer, render_pass, mesh_registry)?;
        let fireflies = Fireflies::new(renderer, render_pass, mesh_registry)?;

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
            .map(|firefly| *firefly.lock().unwrap().as_ref())
            .collect::<Vec<Light>>();
        data.push(self.sun.lock().unwrap().light);
        data.push(self.player.lock().unwrap().light);
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
            .lock().unwrap()
            .frame_finished(keyboard, mouse, camera, time, viewport, socket);
        self.sun.lock().unwrap().frame_finished(time);
        self.fireflies
            .iter_mut()
            .for_each(|firefly| firefly.lock().unwrap().frame_finished(&self.sun.lock().unwrap(), time));
    }
}
