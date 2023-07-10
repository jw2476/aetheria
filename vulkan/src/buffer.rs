use super::Context;
use ash::vk;
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator},
    MemoryLocation,
};
use std::sync::{Arc, Mutex};
use std::{
    ops::{Deref, Drop},
    result::Result,
};

pub struct Buffer {
    pub(crate) buffer: vk::Buffer,
    pub(crate) allocation: Option<Allocation>,
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
        let buffer = unsafe { ctx.device.create_buffer(&create_info, None)? };

        let requirements = unsafe { ctx.device.get_buffer_memory_requirements(buffer) };
        let allocation_info = AllocationCreateDesc {
            name: "buffer",
            requirements,
            location: MemoryLocation::CpuToGpu,
            linear: true,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        };
        let allocation = ctx
            .allocator
            .lock()
            .unwrap()
            .allocate(&allocation_info)
            .unwrap();
        unsafe {
            ctx.device
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?;
        };

        let mut buffer = Self {
            buffer,
            allocation: Some(allocation),
            size: bytes.len(),
            allocator: ctx.allocator.clone(),
        };

        buffer.upload_bytes(&bytes);

        Ok(buffer)
    }

    pub fn upload<T: Into<Vec<u8>>>(&mut self, data: T) {
        let bytes: Vec<u8> = data.into();

        self.upload_bytes(&bytes);
    }

    fn upload_bytes(&mut self, bytes: &[u8]) {
        self.allocation
            .as_mut()
            .unwrap()
            .mapped_slice_mut()
            .unwrap()[..bytes.len()]
            .copy_from_slice(bytes);
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
        self.allocator
            .lock()
            .unwrap()
            .free(self.allocation.take().expect("Vulkan buffer double free"))
            .expect("Failed to free vulkan buffer");
    }
}
