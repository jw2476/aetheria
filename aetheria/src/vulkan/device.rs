use super::{Instance, Surface};
use ash::{extensions::khr, vk};
use bytemuck::cast_slice;
use std::{collections::HashSet, ffi::CStr, ops::Deref, result::Result};
use tracing::info;

pub struct Queue {
    pub(crate) queue: vk::Queue,
    pub index: u32,
}

impl Queue {
    const fn new(queue: vk::Queue, index: u32) -> Self {
        Self { queue, index }
    }
}

impl Deref for Queue {
    type Target = vk::Queue;

    fn deref(&self) -> &Self::Target {
        &self.queue
    }
}

pub struct Queues {
    pub graphics: Queue,
    pub present: Queue,
}

pub struct Extensions {
    pub swapchain: Option<khr::Swapchain>,
}

impl Extensions {
    fn load(instance: &ash::Instance, device: &ash::Device, available: &[&CStr]) -> Self {
        Self {
            swapchain: available
                .iter()
                .find(|ext| **ext == khr::Swapchain::name())
                .map(|_| khr::Swapchain::new(instance, device)),
        }
    }
}

pub struct Device {
    pub(crate) device: ash::Device,
    pub physical: super::instance::PhysicalDevice,
    pub queues: Queues,
    pub extensions: Extensions,
}

impl Device {
    pub unsafe fn new(instance: &Instance, surface: &Surface) -> Result<Self, vk::Result> {
        let physicals = instance.get_physical_devices()?;
        let physical = physicals
            .first()
            .cloned()
            .expect("No device supporting vulkan found");

        let features = vk::PhysicalDeviceFeatures::default();

        let (graphics_family_index, _graphics_family) = physical
            .queue_families
            .iter()
            .enumerate()
            .find(|(_, family)| family.queue_flags.intersects(vk::QueueFlags::GRAPHICS))
            .expect("No graphics queue family");

        let (present_family_index, _present_family) = physical
            .queue_families
            .iter()
            .enumerate()
            .find(|(i, _)| {
                instance
                    .extensions
                    .surface
                    .as_ref()
                    .unwrap()
                    .get_physical_device_surface_support(
                        physical.physical,
                        (*i).try_into().unwrap(),
                        surface.surface,
                    )
                    .unwrap()
            })
            .expect("No present family");

        info!("Found graphics family at index {}", graphics_family_index);
        info!("Found present family at index {}", present_family_index);

        let queue_family_indices = [graphics_family_index, present_family_index];
        let unique_queue_family_indices: HashSet<usize> = HashSet::from_iter(queue_family_indices);

        let queue_priorities = [1.0];
        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = unique_queue_family_indices
            .iter()
            .map(|index| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index((*index).try_into().unwrap())
                    .queue_priorities(&queue_priorities)
                    .build()
            })
            .collect();

        let available_layers = instance.enumerate_device_layer_properties(physical.physical)?;
        let available_extensions =
            instance.enumerate_device_extension_properties(physical.physical)?;

        let available_layer_names: Vec<&CStr> = available_layers
            .iter()
            .map(|layer| CStr::from_bytes_until_nul(cast_slice(&layer.layer_name)).unwrap())
            .collect();

        let available_extension_names: Vec<&CStr> = available_extensions
            .iter()
            .map(|extension| {
                CStr::from_bytes_until_nul(cast_slice(&extension.extension_name)).unwrap()
            })
            .collect();

        let wanted_layers = super::get_wanted_layers();
        let wanted_extensions = get_wanted_extensions();

        let wanted_layers = super::intersection(&wanted_layers, &available_layer_names);
        let wanted_extensions = super::intersection(&wanted_extensions, &available_extension_names);

        info!("Using device layers: {:?}", wanted_layers);
        info!("Using device extesions: {:?}", wanted_extensions);

        let wanted_layers_raw: Vec<*const i8> =
            wanted_layers.iter().map(|name| name.as_ptr()).collect();
        let wanted_extensions_raw: Vec<*const i8> =
            wanted_extensions.iter().map(|name| name.as_ptr()).collect();

        let create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_layer_names(&wanted_layers_raw)
            .enabled_extension_names(&wanted_extensions_raw)
            .enabled_features(&features);

        let device = unsafe { instance.create_device(physical.physical, &create_info, None)? };

        info!("Created vulkan device");

        let graphics = device.get_device_queue(graphics_family_index.try_into().unwrap(), 0);
        let graphics = Queue::new(graphics, graphics_family_index.try_into().unwrap());

        let present = device.get_device_queue(present_family_index.try_into().unwrap(), 0);
        let present = Queue::new(present, present_family_index.try_into().unwrap());

        Ok(Self {
            extensions: Extensions::load(instance, &device, &available_extension_names),
            device,
            physical,
            queues: Queues { graphics, present },
        })
    }
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

fn get_wanted_extensions() -> Vec<&'static CStr> {
    vec![khr::Swapchain::name()]
}
