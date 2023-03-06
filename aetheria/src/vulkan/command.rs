use super::Device;
use ash::{prelude::*, vk};
use std::{ops::Deref, result::Result};

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

impl CommandPool {
    pub fn new(device: &Device) -> Result<Self, vk::Result> {
        let create_info =
            vk::CommandPoolCreateInfo::builder().queue_family_index(device.queues.graphics.index);

        let pool = unsafe { device.create_command_pool(&create_info, None)? };

        Ok(Self { pool })
    }

    pub fn allocate(&self, device: &Device) -> Result<CommandBuffer, vk::Result> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let buffer = unsafe { device.allocate_command_buffers(&allocate_info)?[0] };

        Ok(CommandBuffer { buffer })
    }
}

impl Deref for CommandPool {
    type Target = vk::CommandPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
