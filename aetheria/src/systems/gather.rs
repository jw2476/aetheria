use glam::{IVec2, Quat, UVec2, Vec3};

use crate::{
    camera::Camera,
    components,
    entities::Player,
    input::Keyboard,
    renderer::{RENDER_HEIGHT, RENDER_WIDTH},
    ui::{Element, Rectangle, Region, SizeConstraints},
};
use common::item::Inventory;

use super::{Named, Positioned};

use std::{
    f32::consts::PI,
    sync::{Arc, Mutex, Weak},
};

pub struct System {
    gatherables: Vec<Weak<Mutex<dyn Gatherable>>>,
    player: Option<Weak<Mutex<Player>>>,
}

impl System {
    pub fn new() -> Self {
        Self {
            gatherables: Vec::new(),
            player: None,
        }
    }

    pub fn add_gatherable<T: Gatherable + Sized + 'static>(&mut self, gatherable: Arc<Mutex<T>>) {
        self.gatherables
            .push(Arc::downgrade(&(gatherable as Arc<Mutex<dyn Gatherable>>)))
    }

    pub fn set_player(&mut self, player: Arc<Mutex<Player>>) {
        self.player = Some(Arc::downgrade(&player));
    }

    pub fn frame_finished(
        &mut self,
        camera: &Camera,
        keyboard: &Keyboard,
        scene: &mut Vec<Rectangle>,
        inventory: &mut Inventory,
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
            .gatherables
            .iter()
            .enumerate()
            .filter_map(|(i, gatherable)| gatherable.upgrade().map(|g| (i, g)))
            .filter(|(_, gatherable)| gatherable.lock().unwrap().is_gatherable())
            .map(|(i, gatherable)| {
                (
                    i,
                    (gatherable.lock().unwrap().get_position() - player_position).length(),
                )
            })
            .collect::<Vec<(usize, f32)>>();

        distances.sort_by(|(_, a), (_, b)| a.total_cmp(&b));
        let Some(closest_gatherable) = distances.first() else { return; };

        if closest_gatherable.1 < 50.0 {
            let gatherable = self.gatherables[closest_gatherable.0].upgrade().unwrap();
            let mut gather_widget =
                components::gather::Component::new(&gatherable.lock().unwrap().get_name());
            let size = gather_widget.layout(SizeConstraints {
                min: UVec2::new(0, 0),
                max: UVec2::new(RENDER_WIDTH, RENDER_HEIGHT),
            });

            let gather_origin = IVec2::new(250, 145)
                + IVec2::new(
                    camera_delta.x as i32,
                    (camera_delta.z * 2.0_f32.powf(-0.5)) as i32,
                );
            gather_widget.paint(
                Region {
                    origin: UVec2::new(gather_origin.x as u32, gather_origin.y as u32),
                    size,
                },
                scene,
            );

            if keyboard.is_key_pressed(winit::event::VirtualKeyCode::F) {
                gatherable.lock().unwrap().gather(inventory);
            }
        }
    }
}

pub trait Gatherable: Named + Positioned {
    fn gather(&mut self, inventory: &mut Inventory);
    fn is_gatherable(&self) -> bool;
}
