use std::{
    f32::consts::PI,
    sync::{Arc, Mutex},
};

use ash::vk;
use assets::{ModelRegistry, Transform};
use common::{
    item::{Item, ItemStack},
    net,
};
use glam::{Vec2, Vec3};
use winit::event::VirtualKeyCode;

use crate::{
    camera::Camera,
    data::Data,
    input::{Keyboard, Mouse},
    renderer::Renderer,
    socket::Socket,
    systems::{
        render::{Emissive, Light, RenderObject, Renderable},
        Positioned, Systems,
    },
    time::Time,
};

const PLAYER_SPEED: f32 = 100.0;
const JUMP_HEIGHT: f32 = 100.0;
const JUMP_SPEED: f32 = 4.0;
const DASH_DISTANCE: f32 = 100.0;

#[derive(Clone)]
pub struct Player {
    pub player: RenderObject,
    jump_t: f32,
    pub light: Light,
}

impl Player {
    pub fn new(
        renderer: &mut Renderer,
        systems: &mut Systems,
        model_registry: &mut ModelRegistry,
        transform: Transform,
    ) -> Result<Arc<Mutex<Self>>, vk::Result> {
        let player = RenderObject {
            model: model_registry.load("player.glb"),
            transform,
        };

        let player = Arc::new(Mutex::new(Self {
            player,
            jump_t: 0.0,
            light: Light::new(Vec3::ZERO, 5000.0, Vec3::new(1.0, 1.0, 1.0)),
        }));

        systems.render.add(player.clone());
        systems.render.add_light(player.clone());

        Ok(player)
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
        let old_translation = self.player.transform.translation.clone();

        // Dash
        if keyboard.is_key_pressed(VirtualKeyCode::Space) && self.jump_t >= (PI / 4.0) {
            let mouse_direction = (mouse.position - (viewport / 2.0)).normalize_or_zero();
            let mouse_direction =
                camera.get_rotation() * Vec3::new(mouse_direction.x, 0.0, mouse_direction.y);
            self.player.transform.translation += mouse_direction * DASH_DISTANCE
        }

        // Jump
        if keyboard.is_key_pressed(VirtualKeyCode::Space) && self.jump_t == 0.0 {
            self.jump_t = std::f32::consts::PI - 0.0001;
        }

        self.player.transform.translation.y = self.jump_t.sin().powf(0.6) * JUMP_HEIGHT;
        self.jump_t -= time.delta_seconds() * JUMP_SPEED;
        self.jump_t = self.jump_t.max(0.0);

        // Movement
        let z = keyboard.is_key_down(VirtualKeyCode::W) as i32
            - keyboard.is_key_down(VirtualKeyCode::S) as i32;
        let x = keyboard.is_key_down(VirtualKeyCode::D) as i32
            - keyboard.is_key_down(VirtualKeyCode::A) as i32;
        if x != 0 || z != 0 {
            let delta = Vec3::new(x as f32, 0.0, z as f32).normalize()
                * PLAYER_SPEED
                * time.delta_seconds();
            self.player.transform.translation += camera.get_rotation() * delta;
        }

        self.light.position = self.player.transform.translation + Vec3::new(0.0, 15.0, 0.0);

        if old_translation != self.player.transform.translation {
            let packet = net::server::Packet::Move(net::server::Move {
                position: self.player.transform.translation.clone(),
            });
            socket.send(&packet).unwrap();
        }
    }
}

impl Emissive for Player {
    fn get_lights(&self, data: &Data) -> Vec<Light> {
        if data
            .inventory
            .get_items()
            .iter()
            .find(|stack| stack.item == Item::Lamp)
            .is_some()
        {
            vec![self.light]
        } else {
            Vec::new()
        }
    }
}

impl Renderable for Player {
    fn get_objects(&self) -> Vec<RenderObject> {
        vec![self.player.clone()]
    }
}

impl Positioned for Player {
    fn get_position(&self) -> Vec3 {
        self.player.transform.translation
    }
}
