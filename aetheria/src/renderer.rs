use crate::vulkan::*;
use ash::vk;
use std::ops::Deref;

pub struct Renderer {
    ctx: VulkanContext,

    renderpass: Renderpass,
    pipeline: GraphicsPipeline,
    shaders: Shaders,
    framebuffers: Vec<vk::Framebuffer>,

    render_finished: vk::Semaphore,
    in_flight: vk::Fence,
}

impl Renderer {
    pub fn new(ctx: VulkanContext) -> Result<Self, vk::Result> {
        let renderpass = Renderpass::new(&ctx.device, ctx.swapchain.format)?;

        let vertex_shader = Shader::new(
            &ctx.device,
            include_bytes!("../../assets/shaders/compiled/vertex.spv").to_vec(),
            vk::ShaderStageFlags::VERTEX,
        )?;
        let fragment_shader = Shader::new(
            &ctx.device,
            include_bytes!("../../assets/shaders/compiled/fragment.spv").to_vec(),
            vk::ShaderStageFlags::FRAGMENT,
        )?;
        let shaders = Shaders {
            vertex: Some(vertex_shader),
            fragment: Some(fragment_shader),
        };
        let pipeline = GraphicsPipeline::new(
            &ctx.device,
            &renderpass,
            shaders.clone(),
            ctx.swapchain.extent,
        )?;

        let framebuffers: Vec<vk::Framebuffer> = std::iter::zip(
            ctx.swapchain.images.clone(),
            ctx.swapchain.image_views.clone(),
        )
        .map(|(image, view)| {
            renderpass
                .create_framebuffer(&ctx.device, &image, &view)
                .unwrap()
        })
        .collect();

        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let render_finished =
            unsafe { ctx.device.create_semaphore(&semaphore_info, None).unwrap() };
        let in_flight = unsafe { ctx.device.create_fence(&fence_info, None).unwrap() };

        Ok(Self {
            ctx,
            renderpass,
            pipeline,
            shaders,
            framebuffers,
            render_finished,
            in_flight,
        })
    }

    unsafe fn destroy_swapchain(&mut self) {
        self.ctx.device.device_wait_idle().unwrap();

        self.framebuffers
            .iter()
            .for_each(|framebuffer| self.ctx.device.destroy_framebuffer(*framebuffer, None));
        self.ctx.device.destroy_pipeline(*self.pipeline, None);
        self.ctx
            .device
            .destroy_pipeline_layout(self.pipeline.layout, None);
        self.ctx.device.destroy_render_pass(*self.renderpass, None);
        self.ctx
            .swapchain
            .image_views
            .iter()
            .for_each(|view| self.ctx.device.destroy_image_view(*view, None));
        self.ctx
            .device
            .extensions
            .swapchain
            .as_ref()
            .unwrap()
            .destroy_swapchain(*self.ctx.swapchain, None);
    }

    pub fn recreate_swapchain(&mut self, window: &winit::window::Window) {
        unsafe { self.destroy_swapchain() };

        self.ctx.swapchain = Swapchain::new(
            &self.ctx.instance,
            &self.ctx.surface,
            &self.ctx.device,
            window,
        )
        .expect("Swapchain recreation failed");
        self.renderpass = Renderpass::new(&self.ctx.device, self.ctx.swapchain.format)
            .expect("Swapchain recreation failed");
        self.pipeline = GraphicsPipeline::new(
            &self.ctx.device,
            &self.renderpass,
            self.shaders.clone(),
            self.ctx.swapchain.extent,
        )
        .expect("Swapchain recreation failed");
        self.framebuffers = std::iter::zip(
            self.ctx.swapchain.images.clone(),
            self.ctx.swapchain.image_views.clone(),
        )
        .map(|(image, view)| {
            self.renderpass
                .create_framebuffer(&self.ctx.device, &image, &view)
                .unwrap()
        })
        .collect();
    }

    pub fn render(
        &mut self,
        window: &winit::window::Window,
        vertex_buffer: &Buffer,
        index_buffer: &Buffer,
    ) {
        unsafe {
            self.ctx
                .render(self.in_flight, |ctx, image_available, image_index| {
                    ctx.command_pool.clear(&ctx.device);
                    let cmd = ctx
                        .command_pool
                        .allocate(&ctx.device)
                        .unwrap()
                        .begin(&ctx.device)
                        .unwrap()
                        .begin_renderpass(
                            &ctx.device,
                            &self.renderpass,
                            &self.framebuffers[image_index as usize],
                            ctx.swapchain.extent,
                        )
                        .bind_pipeline(&ctx.device, &self.pipeline)
                        .bind_index_buffer(&ctx.device, index_buffer)
                        .bind_vertex_buffer(&ctx.device, vertex_buffer)
                        .draw(
                            &ctx.device,
                            DrawOptions {
                                vertex_count: (index_buffer.size / 4).try_into().unwrap(),
                                instance_count: 1,
                                ..Default::default()
                            },
                        )
                        .end_renderpass(&ctx.device)
                        .end(&ctx.device)
                        .unwrap();

                    let wait_semaphores = &[image_available];
                    let signal_semaphores = &[self.render_finished];
                    let command_buffers = &[*cmd];
                    let submit_info = vk::SubmitInfo::builder()
                        .wait_semaphores(wait_semaphores)
                        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                        .command_buffers(command_buffers)
                        .signal_semaphores(signal_semaphores);

                    ctx.device
                        .queue_submit(
                            ctx.device.queues.graphics.queue,
                            &[*submit_info],
                            self.in_flight,
                        )
                        .unwrap();

                    self.render_finished
                })
                .unwrap();
        }
    }
}

impl Deref for Renderer {
    type Target = VulkanContext;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}
