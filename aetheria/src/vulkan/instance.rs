use ash::{extensions::khr, prelude::*, vk};
use bytemuck::cast_slice;
use cstr::cstr;
use std::{clone::Clone, collections::HashSet, ffi::CStr, hash::Hash, ops::Deref, result::Result};
use tracing::info;

#[derive(Debug, Clone)]
pub struct PhysicalDeviceProperties {
    properties: vk::PhysicalDeviceProperties,
    pub device_name: String,
}

impl PhysicalDeviceProperties {
    fn new(properties: vk::PhysicalDeviceProperties) -> Self {
        let device_name_raw: &CStr =
            CStr::from_bytes_until_nul(cast_slice(&properties.device_name)).unwrap();
        let device_name = device_name_raw.to_str().unwrap().to_owned();

        Self {
            properties,
            device_name,
        }
    }
}

impl Deref for PhysicalDeviceProperties {
    type Target = vk::PhysicalDeviceProperties;
    fn deref(&self) -> &Self::Target {
        &self.properties
    }
}

#[derive(Debug, Clone)]
pub struct PhysicalDevice {
    pub(crate) physical: vk::PhysicalDevice,
    pub properties: PhysicalDeviceProperties,
    pub queue_families: Vec<vk::QueueFamilyProperties>,
    pub features: vk::PhysicalDeviceFeatures,
}

impl PhysicalDevice {
    unsafe fn new(instance: &Instance, physical: vk::PhysicalDevice) -> Self {
        let properties = instance.get_physical_device_properties(physical);
        let properties = PhysicalDeviceProperties::new(properties);
        let queue_families = instance.get_physical_device_queue_family_properties(physical);
        let features = instance.get_physical_device_features(physical);

        Self {
            physical,
            properties,
            queue_families,
            features,
        }
    }
}

#[derive(Clone)]
pub struct InstanceExtensions {
    pub surface: Option<khr::Surface>,
    pub xlib_surface: Option<khr::XlibSurface>,
}

impl InstanceExtensions {
    pub fn load(entry: &ash::Entry, instance: &ash::Instance, available: &[&CStr]) -> Self {
        Self {
            surface: available
                .iter()
                .find(|ext| **ext == khr::Surface::name())
                .map(|_| khr::Surface::new(entry, instance)),
            xlib_surface: available
                .iter()
                .find(|ext| **ext == khr::XlibSurface::name())
                .map(|_| khr::XlibSurface::new(entry, instance)),
        }
    }
}

#[derive(Clone)]
pub struct Instance {
    instance: ash::Instance,
    pub extensions: InstanceExtensions,
}

impl Instance {
    pub fn new(entry: &ash::Entry) -> Result<Self, vk::Result> {
        let app_info = vk::ApplicationInfo::builder()
            .application_name(cstr!("aetheria"))
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(cstr!("aetheria"))
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::make_api_version(0, 1, 3, 238));

        let available_layers = entry.enumerate_instance_layer_properties()?;
        let available_extensions = entry.enumerate_instance_extension_properties(None)?;

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

        info!("Using instance layers: {:?}", wanted_layers);
        info!("Using instance extensions: {:?}", wanted_extensions);

        let wanted_layers_raw: Vec<*const i8> =
            wanted_layers.iter().map(|name| name.as_ptr()).collect();
        let wanted_extensions_raw: Vec<*const i8> =
            wanted_extensions.iter().map(|name| name.as_ptr()).collect();

        let instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&wanted_layers_raw)
            .enabled_extension_names(&wanted_extensions_raw);

        let instance = unsafe { entry.create_instance(&instance_info, None)? };

        Ok(Self {
            extensions: InstanceExtensions::load(entry, &instance, &available_extension_names),
            instance,
        })
    }

    pub fn get_physical_devices(&self) -> Result<Vec<PhysicalDevice>, vk::Result> {
        let physicals = unsafe { self.enumerate_physical_devices()? };
        unsafe {
            Ok(physicals
                .iter()
                .cloned()
                .map(|physical| PhysicalDevice::new(self, physical))
                .collect())
        }
    }
}

impl Deref for Instance {
    type Target = ash::Instance;
    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

#[cfg(target_os = "linux")]
fn get_wanted_extensions() -> Vec<&'static CStr> {
    vec![khr::Surface::name(), khr::XlibSurface::name()]
}
