use super::{Buffer, Device, Instance, VulkanContext};
use ash::vk;
use gpu_allocator::vulkan::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc, result::Result};

pub struct Resources {
    allocator: Allocator,
    buffers: HashMap<String, Buffer>,
}

impl Resources {
    pub fn new(instance: &Instance, device: &Device) -> Self {
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: (**instance).clone(),
            device: (**device).clone(),
            physical_device: *device.physical,
            debug_settings: Default::default(),
            buffer_device_address: false,
        })
        .expect("Vulkan allocator creation failed");

        Self {
            allocator,
            buffers: HashMap::new(),
        }
    }

    pub fn new_buffer(
        &mut self,
        ctx: &VulkanContext,
        data: &[u8],
        usage: vk::BufferUsageFlags,
        name: &str,
    ) -> Result<(), vk::Result> {
        let mut buffer = Buffer::new(ctx, &mut self.allocator, data.len(), usage)?;
        buffer.upload(data);

        self.buffers.insert(name.to_owned(), buffer);

        Ok(())
    }

    pub fn get_buffer(&mut self, name: &str) -> Option<&Buffer> {
        self.buffers.get(name)
    }

    pub fn free_buffer(&mut self, name: &str) {
        self.allocator
            .free(self.buffers.remove(name).unwrap().allocation);
    }
}

impl Drop for Resources {
    fn drop(&mut self) {
        let buffers: Vec<String> = self.buffers.drain().map(|(name, _)| name).collect();
        for name in buffers {
            self.free_buffer(&name);
        }
    }
}
