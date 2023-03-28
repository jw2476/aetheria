use std::time::Instant;

use bevy_ecs::prelude::*;

#[derive(Resource)]
pub struct Time {
    last_frame: Instant,
    current_frame: Instant,
}

impl Time {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn delta_seconds(&self) -> f32 {
        (self.current_frame - self.last_frame).as_secs_f32()
    }

    pub fn frame_finished(mut time: ResMut<Self>) {
        println!("FPS: {}", 1.0 / time.delta_seconds());

        time.last_frame = time.current_frame;
        time.current_frame = Instant::now();
    }
}

impl Default for Time {
    fn default() -> Self {
        Self {
            last_frame: Instant::now(),
            current_frame: Instant::now(),
        }
    }
}
