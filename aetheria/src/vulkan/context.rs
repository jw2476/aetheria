use super::{
    command::DrawOptions, graphics::Shaders, Buffer, CommandPool, Device, GraphicsPipeline,
    Instance, Renderpass, Shader, Surface, Swapchain,
};
use ash::{prelude::*, vk, Entry};
use bytemuck::cast_slice;
use gpu_allocator::vulkan::*;
use nalgebra::{Vector2, Vector3};
use std::{
    collections::HashMap,
    ops::{Deref, Drop},
};

pub struct VulkanContext {
    pub(crate) instance: Instance,
    surface: Surface,
    pub(crate) device: Device,
    pub(crate) allocator: Allocator,
    swapchain: Swapchain,
    renderpass: Renderpass,
    pipeline: GraphicsPipeline,
    shaders: Shaders,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: CommandPool,
    buffers: HashMap<String, Buffer>,

    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
    in_flight: vk::Fence,
}

impl VulkanContext {
    pub fn new(window: &winit::window::Window) -> Result<Self, vk::Result> {
        let entry = Entry::linked();
        let instance = Instance::new(&entry).expect("Vulkan instance creation failed");
        let surface = Surface::new(&instance, &window).expect("Vulkan surface creation failed");
        let device =
            unsafe { Device::new(&instance, &surface).expect("Vulkan device creation failed") };
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: (*instance).clone(),
            device: (*device).clone(),
            physical_device: *device.physical,
            debug_settings: Default::default(),
            buffer_device_address: false,
        })
        .expect("Vulkan allocator creation failed");

        let swapchain = Swapchain::new(&instance, &surface, &device, &window)
            .expect("Vulkan swapchain creation failed");

        let vertex_shader = Shader::new(
            &device,
            include_bytes!("../../../assets/shaders/compiled/vertex.spv").to_vec(),
            vk::ShaderStageFlags::VERTEX,
        )
        .unwrap();
        let fragment_shader = Shader::new(
            &device,
            include_bytes!("../../../assets/shaders/compiled/fragment.spv").to_vec(),
            vk::ShaderStageFlags::FRAGMENT,
        )
        .unwrap();
        let renderpass =
            Renderpass::new(&device, swapchain.format).expect("Vulkan renderpass creation failed");
        let shaders = Shaders {
            vertex: Some(vertex_shader),
            fragment: Some(fragment_shader),
        };
        let pipeline =
            GraphicsPipeline::new(&device, &renderpass, shaders.clone(), swapchain.extent)
                .expect("Vulkan pipleine creation failed");

        let framebuffers: Vec<ash::vk::Framebuffer> =
            std::iter::zip(swapchain.images.clone(), swapchain.image_views.clone())
                .map(|(image, view)| {
                    renderpass
                        .create_framebuffer(&device, &image, &view)
                        .unwrap()
                })
                .collect();

        let command_pool = CommandPool::new(&device).unwrap();

        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        let image_available = unsafe { device.create_semaphore(&semaphore_info, None).unwrap() };
        let render_finished = unsafe { device.create_semaphore(&semaphore_info, None).unwrap() };
        let in_flight = unsafe { device.create_fence(&fence_info, None).unwrap() };

        let mut ctx = Self {
            instance,
            surface,
            device,
            allocator,
            swapchain,
            renderpass,
            shaders,
            pipeline,
            framebuffers,
            command_pool,
            image_available,
            render_finished,
            in_flight,
            buffers: HashMap::new(),
        };

        let positions = [
            Vector2::new(0.0, -0.5),
            Vector2::new(0.5, 0.5),
            Vector2::new(-0.5, 0.5),
        ];
        let colors = [
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ];

        let vertices: Vec<u8> = std::iter::zip(positions, colors)
            .map(|(position, color)| {
                let mut vertex: Vec<u8> = cast_slice::<f32, u8>(position.as_ref()).to_vec();
                vertex.extend_from_slice(cast_slice::<f32, u8>(color.as_ref()));
                vertex
            })
            .flatten()
            .collect();

        let mut vertex_buffer = Buffer::new(
            &mut ctx,
            vertices.len(),
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        vertex_buffer.upload(&vertices);
        ctx.buffers.insert("vertex".to_owned(), vertex_buffer);

        Ok(ctx)
    }

    unsafe fn destroy_swapchain(&mut self) {
        self.device.device_wait_idle().unwrap();

        self.framebuffers
            .iter()
            .for_each(|framebuffer| self.device.destroy_framebuffer(*framebuffer, None));
        self.device.destroy_pipeline(*self.pipeline, None);
        self.device
            .destroy_pipeline_layout(self.pipeline.layout, None);
        self.device.destroy_render_pass(*self.renderpass, None);
        self.swapchain
            .image_views
            .iter()
            .for_each(|view| self.device.destroy_image_view(*view, None));
        self.device
            .extensions
            .swapchain
            .as_ref()
            .unwrap()
            .destroy_swapchain(*self.swapchain, None);
    }

    pub fn recreate_swapchain(&mut self, window: &winit::window::Window) {
        unsafe { self.destroy_swapchain() };

        self.swapchain = Swapchain::new(&self.instance, &self.surface, &self.device, window)
            .expect("Swapchain recreation failed");
        self.renderpass = Renderpass::new(&self.device, self.swapchain.format)
            .expect("Swapchain recreation failed");
        self.pipeline = GraphicsPipeline::new(
            &self.device,
            &self.renderpass,
            self.shaders.clone(),
            self.swapchain.extent,
        )
        .expect("Swapchain recreation failed");
        self.framebuffers = std::iter::zip(
            self.swapchain.images.clone(),
            self.swapchain.image_views.clone(),
        )
        .map(|(image, view)| {
            self.renderpass
                .create_framebuffer(&self.device, &image, &view)
                .unwrap()
        })
        .collect();
    }

    pub fn render(&mut self, window: &winit::window::Window) {
        let mut frame_rendered = false;
        while !frame_rendered {
            unsafe {
                self.device
                    .wait_for_fences(&[self.in_flight], true, u64::MAX)
                    .unwrap();

                let swapchain_khr = self.device.extensions.swapchain.as_ref().unwrap();

                let acquire_result = swapchain_khr.acquire_next_image(
                    self.swapchain.swapchain,
                    u64::MAX,
                    self.image_available,
                    vk::Fence::null(),
                );

                let image_index = match acquire_result {
                    Ok((image_index, _)) => image_index,
                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                        self.recreate_swapchain(window);
                        continue;
                    }
                    Err(e) => panic!("{}", e),
                };

                self.device.reset_fences(&[self.in_flight]).unwrap();

                self.command_pool.clear(&self.device);
                let cmd = self
                    .command_pool
                    .allocate(&self.device)
                    .unwrap()
                    .begin(&self.device)
                    .unwrap()
                    .begin_renderpass(
                        &self.device,
                        &self.renderpass,
                        &self.framebuffers[image_index as usize],
                        self.swapchain.extent,
                    )
                    .bind_pipeline(&self.device, &self.pipeline)
                    .bind_vertex_buffer(&self.device, &self.buffers.get("vertex").unwrap())
                    .draw(
                        &self.device,
                        DrawOptions {
                            vertex_count: 3,
                            instance_count: 1,
                            ..Default::default()
                        },
                    )
                    .end_renderpass(&self.device)
                    .end(&self.device)
                    .unwrap();

                let wait_semaphores = &[self.image_available];
                let signal_semaphores = &[self.render_finished];
                let command_buffers = &[*cmd];
                let submit_info = vk::SubmitInfo::builder()
                    .wait_semaphores(wait_semaphores)
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                    .command_buffers(command_buffers)
                    .signal_semaphores(signal_semaphores);

                self.device
                    .queue_submit(
                        self.device.queues.graphics.queue,
                        &[*submit_info],
                        self.in_flight,
                    )
                    .unwrap();

                let swapchains = &[self.swapchain.swapchain];
                let image_indices = &[image_index];
                let present_info = vk::PresentInfoKHR::builder()
                    .wait_semaphores(signal_semaphores)
                    .swapchains(swapchains)
                    .image_indices(image_indices);

                let present_result =
                    swapchain_khr.queue_present(self.device.queues.present.queue, &present_info);

                match present_result {
                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                        self.recreate_swapchain(window);
                    }
                    Err(e) => panic!("{}", e),
                    _ => (),
                };

                frame_rendered = true;
            }
        }
    }
}
