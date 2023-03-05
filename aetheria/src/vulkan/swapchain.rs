use super::{Device, Instance, Surface};
use ash::{prelude::*, vk};
use std::ops::Deref;
use winit::window::Window;

pub struct Swapchain {
    pub(crate) swapchain: vk::SwapchainKHR,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
}

impl Swapchain {
    pub fn new(
        instance: &Instance,
        surface: &Surface,
        device: &Device,
        window: &Window,
    ) -> Result<Self, vk::Result> {
        let surface_khr = instance.extensions.surface.as_ref().unwrap();

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

        let swapchain = unsafe {
            device
                .extensions
                .swapchain
                .as_ref()
                .unwrap()
                .create_swapchain(&create_info, None)
                .unwrap()
        };

        Ok(Self {
            swapchain,
            format: format.format,
            extent,
        })
    }
}

impl Deref for Swapchain {
    type Target = vk::SwapchainKHR;

    fn deref(&self) -> &Self::Target {
        &self.swapchain
    }
}
