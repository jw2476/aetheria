use glam::{IVec2, Quat, UVec2, Vec3};

use crate::{
    camera::Camera,
    components,
    data::{inventory::Inventory, Data},
    entities::Player,
    input::Keyboard,
    renderer::{RENDER_HEIGHT, RENDER_WIDTH},
    ui::{Element, Rectangle, Region, SizeConstraints},
};

use super::{Named, Positioned};

use std::{
    f32::consts::PI,
    sync::{Arc, Mutex, Weak},
};

pub struct System {
    interactables: Vec<Weak<Mutex<dyn Interactable>>>,
    player: Option<Weak<Mutex<Player>>>,
}

impl System {
    pub fn new() -> Self {
        Self {
            interactables: Vec::new(),
            player: None,
        }
    }

    pub fn add<T: Interactable + Sized + 'static>(&mut self, interactable: Arc<Mutex<T>>) {
        self.interactables.push(Arc::downgrade(
            &(interactable as Arc<Mutex<dyn Interactable>>),
        ))
    }

    pub fn set_player(&mut self, player: Arc<Mutex<Player>>) {
        self.player = Some(Arc::downgrade(&player));
    }

    pub fn frame_finished(
        &mut self,
        camera: &Camera,
        keyboard: &Keyboard,
        scene: &mut Vec<Rectangle>,
        data: &mut Data,
    ) {
        if self.player.is_none() || self.player.as_ref().unwrap().upgrade().is_none() {
            return;
        }

        let camera_delta =
            Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 2.0 * PI - camera.actual_theta)
                * (camera.target - camera.actual_target);
        let player_position = self
            .player
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap()
            .lock()
            .unwrap()
            .get_position();
        let mut distances = self
            .interactables
            .iter()
            .enumerate()
            .filter_map(|(i, interactable)| interactable.upgrade().map(|g| (i, g)))
            .filter(|(_, interactable)| interactable.lock().unwrap().active())
            .map(|(i, interactable)| {
                (
                    i,
                    (interactable.lock().unwrap().get_position() - player_position).length(),
                )
            })
            .collect::<Vec<(usize, f32)>>();

        distances.sort_by(|(_, a), (_, b)| a.total_cmp(&b));
        let Some(closest) = distances.first() else {
            return;
        };

        if closest.1 < 50.0 {
            let interactable = self.interactables[closest.0].upgrade().unwrap();
            let mut widget =
                components::interact::Component::new(&interactable.lock().unwrap().get_name());
            let size = widget.layout(SizeConstraints {
                min: UVec2::new(0, 0),
                max: UVec2::new(RENDER_WIDTH, RENDER_HEIGHT),
            });

            let origin = IVec2::new(250, 145)
                + IVec2::new(
                    camera_delta.x as i32,
                    (camera_delta.z * 2.0_f32.powf(-0.5)) as i32,
                );
            widget.paint(
                Region {
                    origin: UVec2::new(origin.x as u32, origin.y as u32),
                    size,
                },
                scene,
            );

            if keyboard.is_key_pressed(winit::event::VirtualKeyCode::F) {
                interactable.lock().unwrap().interact(data);
            }
        }
    }
}

pub trait Interactable: Named + Positioned {
    fn interact(&mut self, data: &mut Data);
    fn active(&self) -> bool;
}
