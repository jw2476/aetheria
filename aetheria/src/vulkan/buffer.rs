use super::VulkanContext;
use ash::vk;
use gpu_allocator::{vulkan::*, MemoryLocation};
use std::{
    cell::RefCell,
    ops::{Deref, Drop},
    rc::Rc,
    result::Result,
};

pub struct Buffer {
    pub(crate) buffer: vk::Buffer,
    pub(crate) allocation: Option<Allocation>,
    pub size: usize,
    allocator: Rc<RefCell<Allocator>>,
}

impl Buffer {
    pub fn new(
        ctx: &VulkanContext,
        data: &[u8],
        usage: vk::BufferUsageFlags,
    ) -> Result<Self, vk::Result> {
        let create_info = vk::BufferCreateInfo::builder()
            .size(data.len().try_into().unwrap())
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
            .borrow_mut()
            .allocate(&allocation_info)
            .unwrap();
        unsafe {
            ctx.device
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?
        };

        let mut buffer = Self {
            buffer,
            allocation: Some(allocation),
            size: data.len(),
            allocator: ctx.allocator.clone(),
        };

        buffer.upload(data);

        Ok(buffer)
    }

    pub fn upload(&mut self, data: &[u8]) {
        self.allocation
            .as_mut()
            .unwrap()
            .mapped_slice_mut()
            .unwrap()[..data.len()]
            .copy_from_slice(data);
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
            .borrow_mut()
            .free(self.allocation.take().unwrap());
    }
}
