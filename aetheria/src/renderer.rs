use crate::vulkan::*;
use ash::vk;
use bytemuck::cast_slice;
use glam::{Mat4, Vec2, Vec3};
use std::{
    ops::Deref,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub struct Transform {
    model: Mat4,
    view: Mat4,
    projection: Mat4,
}

impl From<Transform> for Vec<u8> {
    fn from(transform: Transform) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.extend_from_slice(bytemuck::bytes_of(&transform.model));
        buffer.extend_from_slice(bytemuck::bytes_of(&transform.view));
        buffer.extend_from_slice(bytemuck::bytes_of(&transform.projection));

        buffer
    }
}

pub struct Vertex {
    position: Vec2,
    color: Vec3,
}

pub struct Renderer {
    ctx: VulkanContext,

    transform_layout: DescriptorSetLayout,
    renderpass: Renderpass,
    pipeline: GraphicsPipeline,
    shaders: Shaders,
    framebuffers: Vec<vk::Framebuffer>,

    render_finished: vk::Semaphore,
    in_flight: vk::Fence,

    transform_pool: DescriptorPool,
    transform_buffer: Buffer,
    transform_set: DescriptorSet,
}

impl Renderer {
    pub fn new(ctx: VulkanContext) -> Result<Self, vk::Result> {
        let transform_layout = DescriptorSetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::UNIFORM_BUFFER)
            .build()?;

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

        let descriptor_layouts = &[transform_layout.clone()];
        let pipeline = GraphicsPipeline::new(
            &ctx.device,
            &renderpass,
            shaders.clone(),
            ctx.swapchain.extent,
            descriptor_layouts,
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

        let rotation = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as f64;

        let transform = Transform {
            model: Mat4::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), (rotation / 1000.0) as f32),
            view: Mat4::look_at_rh(
                Vec3::new(2.0, 2.0, 2.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ),
            projection: Mat4::perspective_rh(
                45.0_f32.to_radians(),
                ctx.swapchain.extent.width as f32 / ctx.swapchain.extent.height as f32,
                0.01,
                10.0,
            ),
        };

        let mut transform_pool = DescriptorPool::new(&ctx.device, transform_layout.clone(), 1)?;
        let transform_buffer =
            Buffer::new::<Vec<u8>>(&ctx, transform.into(), vk::BufferUsageFlags::UNIFORM_BUFFER)?;
        let transform_set = transform_pool.allocate(&ctx.device)?;
        transform_set.update_buffer(&ctx.device, 0, &transform_buffer);

        Ok(Self {
            ctx,
            transform_layout,
            renderpass,
            pipeline,
            shaders,
            framebuffers,
            render_finished,
            in_flight,
            transform_pool,
            transform_buffer,
            transform_set,
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

        let descriptor_layouts = &[self.transform_layout.clone()];
        self.pipeline = GraphicsPipeline::new(
            &self.ctx.device,
            &self.renderpass,
            self.shaders.clone(),
            self.ctx.swapchain.extent,
            descriptor_layouts,
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
        let rotation = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 1000000;
        let mut rotation = rotation as f32;
        rotation /= 1000.0;

        let transform = Transform {
            model: Mat4::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), rotation as f32),
            view: Mat4::look_at_rh(
                Vec3::new(2.0, 2.0, 2.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ),
            projection: Mat4::perspective_rh(
                45.0_f32.to_radians(),
                self.ctx.swapchain.extent.width as f32 / self.ctx.swapchain.extent.height as f32,
                0.01,
                10.0,
            ),
        };

        self.transform_buffer.upload::<Vec<u8>>(transform.into());

        unsafe {
            let presentation_result =
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
                            .bind_descriptor_set(&ctx.device, &self.pipeline, &self.transform_set)
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
                    });

            match presentation_result {
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self.recreate_swapchain(window),
                Err(e) => panic!("{}", e),
                Ok(_) => (),
            }
        }
    }
}

impl Deref for Renderer {
    type Target = VulkanContext;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}
