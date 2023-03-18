use std::cell::RefCell;
use super::{Context, Device};
use ash::vk;
use std::ops::Deref;
use std::rc::Rc;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};

#[derive(Debug)]
pub struct Image {
    pub(crate) image: vk::Image,
    pub format: vk::Format,
    pub width: u32,
    pub height: u32,

    pub(crate) allocation: Option<Allocation>,
    allocator: Option<Rc<RefCell<Allocator>>>,
}

impl Image {
    pub fn new(ctx: &Context, width: u32, height: u32, format: vk::Format, usage: vk::ImageUsageFlags) -> Result<Self, vk::Result> {
        let create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = unsafe { ctx.device.create_image(&create_info, None)? };

        let requirements = unsafe { ctx.device.get_image_memory_requirements(image) };
        let allocation_info = AllocationCreateDesc {
            name: "image",
            requirements,
            location: MemoryLocation::GpuOnly,
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
                .bind_image_memory(image, allocation.memory(), allocation.offset())?;
        };

        Ok(Self {
            image,
            format,
            width,
            height,
            allocation: Some(allocation),
            allocator: Some(ctx.allocator.clone())
        })
    }

    pub const fn from_image(image: vk::Image, format: vk::Format, width: u32, height: u32) -> Self {
        Self {
            image,
            format,
            width,
            height,
            allocation: None,
            allocator: None
        }
    }

    pub fn create_view_without_context(
        &self,
        device: &Device,
    ) -> Result<vk::ImageView, vk::Result> {
        let create_info = vk::ImageViewCreateInfo::builder()
            .image(**self)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(self.format)
            .components(vk::ComponentMapping::default())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        unsafe { device.create_image_view(&create_info, None) }
    }
}

impl Deref for Image {
    type Target = vk::Image;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if self.allocation.is_none() || self.allocator.is_none() { return }

        self.allocator
            .take()
            .unwrap()
            .borrow_mut()
            .free(self.allocation.take().expect("Vulkan buffer double free"))
            .expect("Failed to free vulkan buffer");
    }
}