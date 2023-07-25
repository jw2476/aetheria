use super::{
    allocator::{Allocation, Allocator},
    Context,
};
use ash::vk::{self, MemoryPropertyFlags};
use std::sync::{Arc, Mutex};
use std::{
    ops::{Deref, Drop},
    result::Result,
};

pub struct Buffer {
    pub(crate) buffer: vk::Buffer,
    pub(crate) allocation: Allocation,
    pub size: usize,
    allocator: Arc<Mutex<Allocator>>,
}

impl Buffer {
    pub fn new<T: Into<Vec<u8>>>(
        ctx: &Context,
        data: T,
        usage: vk::BufferUsageFlags,
    ) -> Result<Self, vk::Result> {
        let bytes: Vec<u8> = data.into();

        let create_info = vk::BufferCreateInfo::builder()
            .size(bytes.len() as u64)
            .usage(usage);

        let (buffer, allocation) = ctx.allocator.lock().unwrap().create_buffer(
            &create_info,
            MemoryPropertyFlags::DEVICE_LOCAL
                | MemoryPropertyFlags::HOST_VISIBLE
                | MemoryPropertyFlags::HOST_COHERENT,
        )?;

        ctx.allocator.lock().unwrap().write(&allocation, &bytes)?;

        Ok(Self {
            buffer,
            allocation,
            size: bytes.len(),
            allocator: ctx.allocator.clone(),
        })
    }

    pub fn upload(&self, bytes: &[u8]) {
        self.allocator
            .lock()
            .unwrap()
            .write(&self.allocation, bytes)
            .expect("Failed to write to buffer");
    }
}

impl Deref for Buffer {
    type Target = vk::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.allocator.lock().unwrap().free(&self.allocation);
    }
}
