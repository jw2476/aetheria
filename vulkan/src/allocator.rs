use crate::{Buffer, Device, Instance};
use ash::vk;
use std::{ffi::c_void, fmt::Debug, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    size: usize,
    offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allocation {
    id: usize,
    region: Region,
}
// Vulkan calls these memory types
#[derive(Clone, Debug)]
pub struct Heap {
    size: usize,
    properties: vk::MemoryPropertyFlags,
    memory: vk::DeviceMemory,
    allocations: Vec<Allocation>,
}

pub struct Allocator {
    device: Arc<Device>,
    heaps: Vec<Heap>,
    next_id: usize,
}

impl Debug for Allocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Allocator")
            .field("heaps", &self.heaps)
            .field("next_id", &self.next_id)
            .finish()
    }
}

impl Allocator {
    pub fn new(instance: &Instance, device: Arc<Device>) -> Result<Self, vk::Result> {
        let properties =
            unsafe { instance.get_physical_device_memory_properties(*device.physical) };
        let heaps = &properties.memory_types[0..properties.memory_type_count as usize];
        let heaps = heaps
            .iter()
            .enumerate()
            .map(|(i, heap)| {
                let alloc_info = vk::MemoryAllocateInfo::builder()
                    .allocation_size(32 * 1024 * 1024) // 32MiB
                    .memory_type_index(i as u32);
                let memory = unsafe {
                    device
                        .allocate_memory(&alloc_info, None)
                        .expect("Failed to allocate memory")
                };

                Heap {
                    size: properties.memory_heaps[heap.heap_index as usize].size as usize,
                    properties: heap.property_flags,
                    memory,
                    allocations: Vec::new(),
                }
            })
            .collect::<Vec<Heap>>();
        Ok(Self {
            device,
            heaps,
            next_id: 0,
        })
    }

    fn find_region(
        size: usize,
        alignment: usize,
        occupied: Vec<Region>,
        end: usize,
    ) -> Option<Region> {
        let mut points = vec![0_usize];
        for region in occupied {
            points.push(region.offset);
            points.push(region.offset + region.size);
        }
        points.push(end);

        let free = points
            .chunks_exact(2)
            .map(|points| {
                let from = points[0];
                let to = points[1];
                Region {
                    offset: from + (from % alignment),
                    size: to - (from + (from % alignment)),
                }
            })
            .collect::<Vec<Region>>();

        for region in free {
            if region.size > size {
                return Some(Region {
                    size,
                    offset: region.offset,
                });
            }
        }

        None
    }

    fn allocate_from_requirements(
        &mut self,
        requirements: vk::MemoryRequirements,
        properties: vk::MemoryPropertyFlags,
    ) -> (vk::DeviceMemory, Allocation) {
        let (_, heap) = self
            .heaps
            .iter_mut()
            .enumerate()
            .filter(|(i, heap)| {
                heap.properties.contains(properties)
                    && (requirements.memory_type_bits & (1 << i)) != 0
            })
            .next()
            .expect("No suitable memory heap");

        let region = Self::find_region(
            requirements.size as usize,
            requirements.alignment as usize,
            heap.allocations
                .iter()
                .map(|alloc| alloc.region)
                .collect::<Vec<Region>>(),
            32 * 1024 * 1024,
        )
        .expect("Cannot find region in heap");

        let allocation = Allocation {
            id: self.next_id,
            region,
        };

        heap.allocations.push(allocation);
        self.next_id += 1;
        (heap.memory, allocation)
    }

    pub fn create_buffer(
        &mut self,
        create_info: &vk::BufferCreateInfo,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, Allocation), vk::Result> {
        let buffer = unsafe { self.device.create_buffer(create_info, None)? };
        let requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };
        let (memory, allocation) = self.allocate_from_requirements(requirements, properties);
        unsafe {
            self.device
                .bind_buffer_memory(buffer, memory, allocation.region.offset as u64)?
        };

        Ok((buffer, allocation))
    }

    pub fn create_image(
        &mut self,
        create_info: &vk::ImageCreateInfo,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Image, Allocation), vk::Result> {
        let image = unsafe { self.device.create_image(create_info, None)? };
        let requirements = unsafe { self.device.get_image_memory_requirements(image) };
        let (memory, allocation) = self.allocate_from_requirements(requirements, properties);
        unsafe {
            self.device
                .bind_image_memory(image, memory, allocation.region.offset as u64)?
        };

        Ok((image, allocation))
    }

    pub fn write(&self, allocation: &Allocation, bytes: &[u8]) -> Result<(), vk::Result> {
        if bytes.len() > allocation.region.size {
            panic!("Buffer overflow with allocation {}", allocation.id)
        }

        let heap = self
            .heaps
            .iter()
            .find(|heap| heap.allocations.contains(allocation))
            .expect(&format!("Can't find allocation with id {}", allocation.id));
        let ptr = unsafe {
            self.device.map_memory(
                heap.memory,
                allocation.region.offset as u64,
                allocation.region.size as u64,
                vk::MemoryMapFlags::empty(),
            )?
        };
        unsafe { ptr.copy_from(bytes.as_ptr() as *const c_void, bytes.len()) };

        unsafe { self.device.unmap_memory(heap.memory) };

        Ok(())
    }

    pub fn free(&mut self, allocation: &Allocation) {
        let heap = self
            .heaps
            .iter_mut()
            .find(|heap| heap.allocations.contains(allocation))
            .expect(&format!("Double free of allocation {}", allocation.id));
        heap.allocations.remove(
            heap.allocations
                .iter()
                .position(|alloc| alloc.id == allocation.id)
                .unwrap(),
        );
    }
}
