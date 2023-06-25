use std::time::Instant;
use ash::vk;
use bytemuck::cast_slice;
use vulkan::Buffer;

use crate::renderer::Renderer;

pub struct Time {
    last_frame: Instant,
    current_frame: Instant,
    pub time: f32,
}

impl Time {
    pub fn new() -> Result<Self, vk::Result> {
        let time = Self {
            last_frame: Instant::now(),
            current_frame: Instant::now(),
            time: 0.0,
        };
        Ok(time)
    }

    pub fn delta_seconds(&self) -> f32 {
        (self.current_frame - self.last_frame).as_secs_f32()
    }

    pub fn update_buffer(&self, buffer: &mut Buffer) {
        let delta = self.delta_seconds();
        let data = &[self.time, delta];
        let data = cast_slice::<f32, u8>(data);
        buffer.upload(data);
    }

    pub fn frame_finished(&mut self) {
        let delta = self.delta_seconds();
        self.time += delta;

        println!("FPS: {}", 1.0 / self.delta_seconds());

        self.last_frame = self.current_frame;
        self.current_frame = Instant::now();
    }
}
