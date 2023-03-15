use super::{
    graphics::Shaders, Buffer, Device, Pipeline,
    Instance, Renderpass, Shader, Surface, Swapchain, command
};
use ash::{vk, Entry};
use gpu_allocator::{vulkan::{Allocator, AllocatorCreateDesc}, AllocatorDebugSettings};
use std::{cell::RefCell, rc::Rc};

pub struct Context {
    pub instance: Instance,
    pub surface: Surface,
    pub device: Device,
    pub swapchain: Swapchain,
    pub command_pool: command::Pool,

    image_available: vk::Semaphore,

    pub(crate) allocator: Rc<RefCell<Allocator>>,
}

impl Context {
    pub fn new(window: &winit::window::Window) -> Self {
        let entry = Entry::linked();
        let instance = Instance::new(&entry).expect("Vulkan instance creation failed");
        let surface = Surface::new(&instance, window).expect("Vulkan surface creation failed");
        let device =
            unsafe { Device::new(&instance, &surface).expect("Vulkan device creation failed") };

        let swapchain = Swapchain::new(&instance, &surface, &device, window)
            .expect("Vulkan swapchain creation failed");

        let command_pool = command::Pool::new(&device).unwrap();

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

        let ctx = Self {
            instance,
            surface,
            device,
            swapchain,
            command_pool,
            image_available,
            allocator: Rc::new(RefCell::new(allocator)),
        };

        ctx
    }

    pub unsafe fn render<F>(&mut self, in_flight: vk::Fence, callback: F) -> Result<(), vk::Result>
    where
        F: Fn(&mut Self, vk::Semaphore, u32) -> vk::Semaphore,
    {
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

            let render_finished = callback(self, self.image_available, image_index);

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
