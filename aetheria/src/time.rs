use std::time::Instant;
use ash::vk;
use bytemuck::cast_slice;
use vulkan::Buffer;

use crate::renderer::Renderer;

pub struct Time {
    last_frame: Instant,
    current_frame: Instant,
    pub time: f32,
    pub buffer: Buffer,
}

impl Time {
    pub fn new(renderer: &Renderer) -> Result<Self, vk::Result> {
        let time = Self {
            last_frame: Instant::now(),
            current_frame: Instant::now(),
            time: 0.0,
            buffer: Buffer::new(&renderer.ctx, cast_slice::<f32, u8>(&[0_f32; 2]), vk::BufferUsageFlags::UNIFORM_BUFFER)?
        };
        renderer.set_time(&time);
        Ok(time)
    }

    pub fn delta_seconds(&self) -> f32 {
        (self.current_frame - self.last_frame).as_secs_f32()
    }

    pub fn frame_finished(&mut self) {
        let delta = self.delta_seconds();
        self.time += delta;

        println!("FPS: {}", 1.0 / self.delta_seconds());
        let buffer = [self.time, delta];
        self.buffer.upload(cast_slice::<f32, u8>(&buffer));

        self.last_frame = self.current_frame;
        self.current_frame = Instant::now();
    }
}
