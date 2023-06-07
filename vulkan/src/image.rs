use super::{Buffer, Context, Device, Pool};
use crate::command::TransitionLayoutOptions;
use crate::Set;
use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;
use std::cell::OnceCell;
use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Image {
    pub(crate) image: vk::Image,
    pub format: vk::Format,
    pub width: u32,
    pub height: u32,

    pub(crate) allocation: Option<Allocation>,
    allocator: Option<Arc<Mutex<Allocator>>>,
}

impl Image {
    pub fn new(
        ctx: &Context,
        width: u32,
        height: u32,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
    ) -> Result<Arc<Self>, vk::Result> {
        let create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
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
            .lock()
            .unwrap()
            .allocate(&allocation_info)
            .unwrap();
        unsafe {
            ctx.device
                .bind_image_memory(image, allocation.memory(), allocation.offset())?;
        };

        Ok(Arc::new(Self {
            image,
            format,
            width,
            height,
            allocation: Some(allocation),
            allocator: Some(ctx.allocator.clone()),
        }))
    }

    pub fn from_image(image: vk::Image, format: vk::Format, width: u32, height: u32) -> Arc<Self> {
        Arc::new(Self {
            image,
            format,
            width,
            height,
            allocation: None,
            allocator: None,
        })
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

    pub fn create_view(&self, ctx: &Context) -> Result<vk::ImageView, vk::Result> {
        let aspect_mask = if self.format == vk::Format::D32_SFLOAT {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::COLOR
        };

        let create_info = vk::ImageViewCreateInfo::builder()
            .image(**self)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(self.format)
            .components(vk::ComponentMapping::default())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        unsafe { ctx.device.create_image_view(&create_info, None) }
    }

    pub fn create_sampler(
        &self,
        ctx: &Context,
        mag_filter: vk::Filter,
        min_filter: vk::Filter,
    ) -> Result<vk::Sampler, vk::Result> {
        let create_info = vk::SamplerCreateInfo::builder()
            .mag_filter(mag_filter)
            .min_filter(min_filter)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::MIRRORED_REPEAT)
            .address_mode_v(vk::SamplerAddressMode::MIRRORED_REPEAT)
            .address_mode_w(vk::SamplerAddressMode::MIRRORED_REPEAT)
            .mip_lod_bias(0.0)
            .anisotropy_enable(true)
            .max_anisotropy(ctx.device.physical.properties.limits.max_sampler_anisotropy)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .min_lod(0.0)
            .max_lod(0.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false);

        unsafe { ctx.device.create_sampler(&create_info, None) }
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
        if self.allocation.is_none() || self.allocator.is_none() {
            return;
        }

        self.allocator
            .take()
            .unwrap()
            .lock()
            .unwrap()
            .free(self.allocation.take().expect("Vulkan buffer double free"))
            .expect("Failed to free vulkan buffer");
    }
}

pub struct Texture {
    pub image: Arc<Image>,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
}

impl Deref for Texture {
    type Target = Image;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

impl Texture {
    pub const WHITE: OnceCell<Self> = OnceCell::new();

    pub fn new(ctx: &mut Context, bytes: &[u8]) -> Result<Self, vk::Result> {
        let (header, data) = qoi::decode_to_vec(bytes).unwrap();

        Self::new_bytes(ctx, &data, header.width, header.height)
    }

    pub fn new_bytes(
        ctx: &mut Context,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Self, vk::Result> {
        let texture_buffer = Buffer::new(ctx, data, vk::BufferUsageFlags::TRANSFER_SRC)?;

        let image = Image::new(
            ctx,
            width,
            height,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        )?;
        let view = image.create_view(ctx)?;
        let sampler = image.create_sampler(ctx, vk::Filter::LINEAR, vk::Filter::LINEAR)?;

        ctx.command_pool
            .allocate()
            .unwrap()
            .begin()
            .unwrap()
            .transition_image_layout(
                &image,
                &TransitionLayoutOptions {
                    old: vk::ImageLayout::UNDEFINED,
                    new: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    source_access: vk::AccessFlags::empty(),
                    destination_access: vk::AccessFlags::TRANSFER_WRITE,
                    source_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                    destination_stage: vk::PipelineStageFlags::TRANSFER,
                },
            )
            .copy_buffer_to_image(&texture_buffer, &image)
            .transition_image_layout(
                &image,
                &TransitionLayoutOptions {
                    old: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    new: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    source_access: vk::AccessFlags::TRANSFER_WRITE,
                    destination_access: vk::AccessFlags::SHADER_READ,
                    source_stage: vk::PipelineStageFlags::TRANSFER,
                    destination_stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
                },
            )
            .submit()
            .unwrap();

        Ok(Self {
            image,
            view,
            sampler,
        })
    }

    pub fn from_image(ctx: &Context, image: Arc<Image>, mag_filter: vk::Filter, min_filter: vk::Filter) -> Result<Self, vk::Result> {
        let view = image.create_view(ctx)?;
        let sampler = image.create_sampler(ctx, mag_filter, min_filter)?;
        Ok(Self {
            image,
            view,
            sampler
        })
    }
}
