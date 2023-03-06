use super::Device;
use ash::{prelude::*, vk};
use std::ops::Deref;

pub struct CommandBuffer {
    pub(crate) buffer: vk::CommandBuffer,
}

impl Deref for CommandBuffer {
    type Target = vk::CommandBuffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

pub struct CommandPool {
    pub(crate) pool: vk::CommandPool,
}

impl CommandPool {}

impl Deref for CommandPool {
    type Target = vk::CommandPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
