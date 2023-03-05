use ash::vk;
use std::ops::Deref;
use super::Instance;

pub struct Queue {
    queue: vk::Queue,
    index: u32,
}

impl Queue {
    fn new(queue: vk::Queue, index: u32) -> Self {
        Self { queue, index }
    }
}

impl Deref for Queue {
    type Target = vk::Queue;

    fn deref(&self) -> &Self::Target {
        &self.queue
    }
}

pub struct Device {
    device: ash::Device,
    queues: Vec<Queue>,
}

impl Device {
    pub fn new(instance: &Instance)
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
