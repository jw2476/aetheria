use super::VulkanContext;
use ash::vk;
use gpu_allocator::{vulkan::*, MemoryLocation};
use std::{
    ops::{Deref, Drop},
    result::Result,
};

pub struct Buffer {
    pub(crate) buffer: vk::Buffer,
    pub(crate) allocation: Allocation,
    pub size: usize,
}

impl Buffer {
    pub fn new(
        ctx: &mut VulkanContext,
        size: usize,
        usage: vk::BufferUsageFlags,
    ) -> Result<Self, vk::Result> {
        let create_info = vk::BufferCreateInfo::builder()
            .size(size.try_into().unwrap())
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

        let allocation = ctx.allocator.allocate(&allocation_info).unwrap();
        unsafe {
            ctx.device
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?
        };

        Ok(Self {
            buffer,
            allocation,
            size,
        })
    }
}

impl Deref for Buffer {
    type Target = vk::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
