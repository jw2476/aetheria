use super::{Device, Image, Instance, Surface};
use ash::{prelude::*, vk};
use std::ops::Deref;
use winit::window::Window;

#[derive(Clone, Debug)]
pub struct Swapchain {
    pub(crate) swapchain: vk::SwapchainKHR,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub images: Vec<Image>,
    pub image_views: Vec<vk::ImageView>,
}

impl Swapchain {
    pub fn new(
        instance: &Instance,
        surface: &Surface,
        device: &Device,
        window: &Window,
    ) -> Result<Self, vk::Result> {
        let surface_khr = instance.extensions.surface.as_ref().unwrap();
        let swapchain_khr = device.extensions.swapchain.as_ref().unwrap();

        let capabilities = unsafe {
            surface_khr
                .get_physical_device_surface_capabilities(device.physical.physical, surface.surface)
                .unwrap()
        };

        let formats = unsafe {
            surface_khr
                .get_physical_device_surface_formats(device.physical.physical, surface.surface)
                .unwrap()
        };

        let present_modes = unsafe {
            surface_khr
                .get_physical_device_surface_present_modes(
                    device.physical.physical,
                    surface.surface,
                )
                .unwrap()
        };

        let format = formats
            .iter()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_SRGB
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or(formats.first().unwrap());

        let present_mode = present_modes
            .iter()
            .copied()
            .find(|present_mode| *present_mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        let extent = if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: window.inner_size().width,
                height: window.inner_size().height,
            }
        };

        let image_count = if capabilities.max_image_count == 0
            || capabilities.min_image_count + 1 < capabilities.max_image_count
        {
            capabilities.min_image_count + 1
        } else {
            capabilities.min_image_count
        };

        let (sharing_mode, queue_family_indices) =
            if device.queues.graphics.index == device.queues.present.index {
                (vk::SharingMode::EXCLUSIVE, Vec::new())
            } else {
                (
                    vk::SharingMode::CONCURRENT,
                    vec![device.queues.graphics.index, device.queues.present.index],
                )
            };

        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(sharing_mode)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        let swapchain = unsafe { swapchain_khr.create_swapchain(&create_info, None).unwrap() };

        let images = unsafe { swapchain_khr.get_swapchain_images(swapchain).unwrap() };
        let images: Vec<Image> = images
            .iter()
            .copied()
            .map(|image| Image::from_image(image, extent.width, extent.height))
            .collect();

        let image_views = images
            .iter()
            .copied()
            .map(|image| {
                let create_info = vk::ImageViewCreateInfo::builder()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .components(vk::ComponentMapping::default())
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });

                unsafe { device.create_image_view(&create_info, None).unwrap() }
            })
            .collect();

        Ok(Self {
            swapchain,
            format: format.format,
            extent,
            images,
            image_views,
        })
    }
}

impl Deref for Swapchain {
    type Target = vk::SwapchainKHR;

    fn deref(&self) -> &Self::Target {
        &self.swapchain
    }
}
