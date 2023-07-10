use super::{command, Device, Instance, Surface, Swapchain};
use ash::{vk, Entry};
use gpu_allocator::{
    vulkan::{Allocator, AllocatorCreateDesc},
    AllocatorDebugSettings,
};
use std::sync::{Arc, Mutex};

pub struct Context {
    pub instance: Instance,
    pub surface: Surface,
    pub device: Arc<Device>,
    pub swapchain: Swapchain,
    pub command_pool: command::Pool,

    pub image_available: vk::Semaphore,

    pub(crate) allocator: Arc<Mutex<Allocator>>,
}

impl Context {
    pub fn new(window: &winit::window::Window) -> Self {
        let entry = Entry::linked();
        let instance = Instance::new(&entry).expect("Vulkan instance creation failed");
        let surface = Surface::new(&instance, window).expect("Vulkan surface creation failed");
        let device = unsafe {
            Arc::new(Device::new(&instance, &surface).expect("Vulkan device creation failed"))
        };

        let swapchain = Swapchain::new(&instance, &surface, &device, window)
            .expect("Vulkan swapchain creation failed");

        let command_pool = command::Pool::new(device.clone()).unwrap();

        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let image_available = unsafe { device.create_semaphore(&semaphore_info, None).unwrap() };

        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: (*instance).clone(),
            device: (*device).clone(),
            physical_device: *device.physical,
            debug_settings: AllocatorDebugSettings::default(),
            buffer_device_address: false,
        })
        .unwrap();

        Self {
            instance,
            surface,
            device,
            swapchain,
            command_pool,
            image_available,
            allocator: Arc::new(Mutex::new(allocator)),
        }
    }

    pub unsafe fn start_frame(&mut self, in_flight: vk::Fence) -> Result<u32, vk::Result> {
        unsafe {
            self.device
                .wait_for_fences(&[in_flight], true, u64::MAX)
                .unwrap();

            let image_index = self
                .device
                .extensions
                .swapchain
                .as_ref()
                .unwrap()
                .acquire_next_image(
                    self.swapchain.swapchain,
                    u64::MAX,
                    self.image_available,
                    vk::Fence::null(),
                )?
                .0;

            self.device.reset_fences(&[in_flight]).unwrap();

            Ok(image_index)
        }
    }

    pub unsafe fn end_frame(
        &self,
        image_index: u32,
        render_finished: vk::Semaphore,
    ) -> Result<(), vk::Result> {
        unsafe {
            let signal_semaphores = &[render_finished];
            let swapchains = &[self.swapchain.swapchain];
            let image_indices = &[image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(signal_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);

            self.device
                .extensions
                .swapchain
                .as_ref()
                .unwrap()
                .queue_present(self.device.queues.present.queue, &present_info)?;
        }

        Ok(())
    }
}
