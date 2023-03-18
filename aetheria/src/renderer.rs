use crate::vulkan::{Buffer, Context, DrawOptions, Image, Pipeline, Pool, Renderpass, Set, SetLayout, SetLayoutBuilder, Shader, Shaders, Swapchain};
use ash::vk;
use glam::{Mat4, Vec3};
use std::{
    ops::Deref,
    time::{SystemTime, UNIX_EPOCH},
};
use std::fs::File;

use crate::include_bytes_align_as;
use crate::vulkan::command::TransitionLayoutOptions;

pub struct Transform {
    model: Mat4,
    view: Mat4,
    projection: Mat4,
}

impl From<Transform> for Vec<u8> {
    fn from(transform: Transform) -> Self {
        let mut buffer = Self::new();
        buffer.extend_from_slice(bytemuck::bytes_of(&transform.model));
        buffer.extend_from_slice(bytemuck::bytes_of(&transform.view));
        buffer.extend_from_slice(bytemuck::bytes_of(&transform.projection));

        buffer
    }
}

pub struct Renderer {
    ctx: Context,

    transform_layout: SetLayout,
    renderpass: Renderpass,
    pipeline: Pipeline,
    shaders: Shaders,
    framebuffers: Vec<vk::Framebuffer>,

    render_finished: vk::Semaphore,
    in_flight: vk::Fence,

    transform_pool: Pool,
    transform_buffer: Option<Buffer>,
    transform_set: Set,

    texture: Option<Image>
}

impl Renderer {
    pub fn new(ctx: Context) -> Result<Self, vk::Result> {
        let transform_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::UNIFORM_BUFFER)
            .build()?;

        let renderpass = Renderpass::new(&ctx.device, ctx.swapchain.format)?;

        let vertex_shader = Shader::new(
            &ctx.device,
            include_bytes_align_as!(u32, "../../assets/shaders/compiled/vertex.spv"),
            vk::ShaderStageFlags::VERTEX,
        )?;
        let fragment_shader = Shader::new(
            &ctx.device,
            include_bytes_align_as!(u32, "../../assets/shaders/compiled/fragment.spv"),
            vk::ShaderStageFlags::FRAGMENT,
        )?;
        let shaders = Shaders {
            vertex: Some(vertex_shader),
            fragment: Some(fragment_shader),
        };

        let descriptor_layouts = &[transform_layout.clone()];
        let pipeline = Pipeline::new(
            &ctx.device,
            &renderpass,
            shaders.clone(),
            ctx.swapchain.extent,
            descriptor_layouts,
        )?;

        let framebuffers: Vec<vk::Framebuffer> = std::iter::zip(
            &ctx.swapchain.images,
            &ctx.swapchain.image_views,
        )
        .map(|(image, view)| {
            renderpass
                .create_framebuffer(&ctx.device, &image, view)
                .unwrap()
        })
        .collect();

        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let render_finished =
            unsafe { ctx.device.create_semaphore(&semaphore_info, None).unwrap() };
        let in_flight = unsafe { ctx.device.create_fence(&fence_info, None).unwrap() };

        let transform = Transform {
            model: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            projection: Mat4::IDENTITY,
        };

        let mut transform_pool = Pool::new(&ctx.device, transform_layout.clone(), 1)?;
        let transform_set = transform_pool.allocate(&ctx.device)?;

        let mut renderer = Self {
            ctx,
            transform_layout,
            renderpass,
            pipeline,
            shaders,
            framebuffers,
            render_finished,
            in_flight,
            transform_pool,
            transform_buffer: None,
            transform_set,
            texture: None
        };

        renderer.transform_buffer =
            Some(Buffer::new::<Vec<u8>>(&renderer.ctx, transform.into(), vk::BufferUsageFlags::UNIFORM_BUFFER)?);
        renderer.transform_set.update_buffer(&renderer.ctx.device, 0, renderer.transform_buffer.as_ref().unwrap());

        let (header, data) = qoi::decode_to_vec(include_bytes!("../../assets/textures/compiled/texture.qoi")).unwrap();
        let texture_buffer = Buffer::new::<Vec<u8>>(&renderer.ctx, data, vk::BufferUsageFlags::TRANSFER_SRC)?;
        renderer.texture = Some(Image::new(&renderer.ctx, header.width, header.height, vk::Format::R8G8B8A8_SRGB, vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)?);
        renderer.ctx.command_pool.allocate(&renderer.ctx.device)
            .unwrap()
            .begin()
            .unwrap()
            .transition_image_layout(renderer.texture.as_ref().unwrap(), &TransitionLayoutOptions {
                old: vk::ImageLayout::UNDEFINED,
                new: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                source_access: vk::AccessFlags::empty(),
                destination_access: vk::AccessFlags::TRANSFER_WRITE,
                source_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                destination_stage: vk::PipelineStageFlags::TRANSFER,
            })
            .copy_buffer_to_image(&texture_buffer, renderer.texture.as_ref().unwrap())
            .transition_image_layout(renderer.texture.as_ref().unwrap(), &TransitionLayoutOptions {
                old: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                source_access: vk::AccessFlags::TRANSFER_WRITE,
                destination_access: vk::AccessFlags::SHADER_READ,
                source_stage: vk::PipelineStageFlags::TRANSFER,
                destination_stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
            })
            .submit()
            .unwrap();

        Ok(renderer)
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
        self.pipeline = Pipeline::new(
            &self.ctx.device,
            &self.renderpass,
            self.shaders.clone(),
            self.ctx.swapchain.extent,
            descriptor_layouts,
        )
        .expect("Swapchain recreation failed");
        self.framebuffers = std::iter::zip(
            &self.ctx.swapchain.images,
            &self.ctx.swapchain.image_views,
        )
        .map(|(image, view)| {
            self.renderpass
                .create_framebuffer(&self.ctx.device, image, view)
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
            % 1_000_000;

        #[allow(clippy::cast_precision_loss)]
        let mut rotation = rotation as f32;
        rotation /= 1000.0;

        #[allow(clippy::cast_precision_loss)]
        let mut transform = Transform {
            model: Mat4::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), rotation),
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

        transform.projection.col_mut(1)[1] *= -1.0;

        self.transform_buffer.as_mut().unwrap().upload::<Vec<u8>>(transform.into());

        unsafe {
            let presentation_result =
                self.ctx
                    .render(self.in_flight, |ctx, image_available, image_index| {
                        ctx.command_pool.clear(&ctx.device);
                        let cmd = ctx
                            .command_pool
                            .allocate(&ctx.device)
                            .unwrap()
                            .begin()
                            .unwrap()
                            .begin_renderpass(
                                &self.renderpass,
                                self.framebuffers[image_index as usize],
                                ctx.swapchain.extent,
                            )
                            .bind_pipeline(&self.pipeline)
                            .bind_descriptor_set(&self.pipeline, &self.transform_set)
                            .bind_index_buffer(index_buffer)
                            .bind_vertex_buffer(vertex_buffer)
                            .draw(
                                DrawOptions {
                                    vertex_count: (index_buffer.size / 4).try_into().unwrap(),
                                    instance_count: 1,
                                    ..Default::default()
                                },
                            )
                            .end_renderpass()
                            .end()
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
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}
