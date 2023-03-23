use ash::vk;
use ecs::{Component, Entity, Event, System};
use ecs_macros::Component;
use glam::{Mat4, Vec3};
use std::fs::File;
use std::rc::Rc;
use std::{
    ops::Deref,
    time::{SystemTime, UNIX_EPOCH},
};
use vulkan::{
    command::TransitionLayoutOptions, Buffer, Context, DrawOptions, Image, Pipeline, Pool,
    Renderpass, Set, SetLayout, SetLayoutBuilder, Shader, Shaders, Swapchain, Texture,
};
use winit::window::Window;

use crate::include_bytes_align_as;

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

#[derive(Component)]
pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
}

pub struct Renderer {
    ctx: Context,
    window: Rc<Window>,

    renderpass: Renderpass,
    pipeline: Pipeline,
    shaders: Shaders,
    framebuffers: Vec<vk::Framebuffer>,

    render_finished: vk::Semaphore,
    in_flight: vk::Fence,

    transform_layout: SetLayout,
    transform_pool: Pool,
    transform_buffer: Option<Buffer>,
    transform_set: Set,

    texture: Option<Texture>,
    texture_layout: SetLayout,
    texture_pool: Pool,
    texture_set: Set,

    depth_image: Image,
    depth_view: vk::ImageView,
}

impl Renderer {
    pub fn new(ctx: Context, window: Rc<Window>) -> Result<Self, vk::Result> {
        let transform_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::UNIFORM_BUFFER)
            .build()?;

        let texture_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .build()?;

        let depth_image = Image::new(
            &ctx,
            ctx.swapchain.extent.width,
            ctx.swapchain.extent.height,
            vk::Format::D32_SFLOAT,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        )?;
        let depth_view = depth_image.create_view(&ctx)?;

        let renderpass =
            Renderpass::new(&ctx.device, ctx.swapchain.format, vk::Format::D32_SFLOAT)?;

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

        let descriptor_layouts = &[transform_layout.clone(), texture_layout.clone()];
        let pipeline = Pipeline::new(
            &ctx.device,
            &renderpass,
            shaders.clone(),
            ctx.swapchain.extent,
            descriptor_layouts,
        )?;

        let framebuffers: Vec<vk::Framebuffer> =
            std::iter::zip(&ctx.swapchain.images, &ctx.swapchain.image_views)
                .map(|(image, &view)| {
                    renderpass
                        .create_framebuffer(
                            &ctx.device,
                            image.width,
                            image.height,
                            &[view, depth_view],
                        )
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

        let mut texture_pool = Pool::new(&ctx.device, texture_layout.clone(), 1)?;
        let texture_set = texture_pool.allocate(&ctx.device)?;

        let mut renderer = Self {
            ctx,
            window,
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
            texture: None,
            texture_layout,
            texture_pool,
            texture_set,
            depth_image,
            depth_view,
        };

        renderer.transform_buffer = Some(Buffer::new::<Vec<u8>>(
            &renderer.ctx,
            transform.into(),
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?);
        renderer.transform_set.update_buffer(
            &renderer.ctx.device,
            0,
            renderer.transform_buffer.as_ref().unwrap(),
        );

        renderer.write_texture()?;

        Ok(renderer)
    }

    fn write_texture(&mut self) -> Result<(), vk::Result> {
        let (header, data) =
            qoi::decode_to_vec(include_bytes!("../../assets/textures/compiled/texture.qoi"))
                .unwrap();
        let texture_buffer =
            Buffer::new::<Vec<u8>>(&self.ctx, data, vk::BufferUsageFlags::TRANSFER_SRC)?;

        self.texture = Some(
            Image::new(
                &self.ctx,
                header.width,
                header.height,
                vk::Format::R8G8B8A8_SRGB,
                vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            )?
            .into_texture(&self.ctx)?,
        );

        self.ctx
            .command_pool
            .allocate(&self.ctx.device)
            .unwrap()
            .begin()
            .unwrap()
            .transition_image_layout(
                self.texture.as_ref().unwrap(),
                &TransitionLayoutOptions {
                    old: vk::ImageLayout::UNDEFINED,
                    new: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    source_access: vk::AccessFlags::empty(),
                    destination_access: vk::AccessFlags::TRANSFER_WRITE,
                    source_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                    destination_stage: vk::PipelineStageFlags::TRANSFER,
                },
            )
            .copy_buffer_to_image(&texture_buffer, self.texture.as_ref().unwrap())
            .transition_image_layout(
                self.texture.as_ref().unwrap(),
                &TransitionLayoutOptions {
                    old: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    new: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    source_access: vk::AccessFlags::TRANSFER_WRITE,
                    destination_access: vk::AccessFlags::SHADER_READ,
                    source_stage: vk::PipelineStageFlags::TRANSFER,
                    destination_stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
                },
            )
            .submit()
            .unwrap();

        self.texture_set.update_texture(
            &self.ctx.device,
            0,
            self.texture.as_ref().unwrap(),
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        Ok(())
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

    pub fn recreate_swapchain(&mut self) -> Result<(), vk::Result> {
        unsafe { self.destroy_swapchain() };

        self.ctx.swapchain = Swapchain::new(
            &self.ctx.instance,
            &self.ctx.surface,
            &self.ctx.device,
            &self.window,
        )?;

        self.depth_image = Image::new(
            &self.ctx,
            self.ctx.swapchain.extent.width,
            self.ctx.swapchain.extent.height,
            vk::Format::D32_SFLOAT,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        )?;
        self.depth_view = self.depth_image.create_view(&self.ctx)?;

        self.renderpass = Renderpass::new(
            &self.ctx.device,
            self.ctx.swapchain.format,
            vk::Format::D32_SFLOAT,
        )?;

        let descriptor_layouts = &[self.transform_layout.clone(), self.texture_layout.clone()];
        self.pipeline = Pipeline::new(
            &self.ctx.device,
            &self.renderpass,
            self.shaders.clone(),
            self.ctx.swapchain.extent,
            descriptor_layouts,
        )?;

        self.framebuffers =
            std::iter::zip(&self.ctx.swapchain.images, &self.ctx.swapchain.image_views)
                .map(|(image, &view)| {
                    self.renderpass
                        .create_framebuffer(
                            &self.ctx.device,
                            image.width,
                            image.height,
                            &[view, self.depth_view],
                        )
                        .unwrap()
                })
                .collect();

        Ok(())
    }

    pub fn render(&mut self, vertex_buffer: &Buffer, index_buffer: &Buffer) {
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

        self.transform_buffer
            .as_mut()
            .unwrap()
            .upload::<Vec<u8>>(transform.into());

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
                            .bind_descriptor_set(&self.pipeline, 0, &self.transform_set)
                            .bind_descriptor_set(&self.pipeline, 1, &self.texture_set)
                            .bind_index_buffer(index_buffer)
                            .bind_vertex_buffer(vertex_buffer)
                            .draw(DrawOptions {
                                vertex_count: (index_buffer.size / 4).try_into().unwrap(),
                                instance_count: 1,
                                ..Default::default()
                            })
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
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self
                    .recreate_swapchain()
                    .expect("Swapchain recreation failed"),
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

impl System for Renderer {
    fn get_requirements(&self) -> u128 {
        Mesh::id()
    }

    fn run(&mut self, entity: &mut Entity) {
        let mesh: &Mesh = entity.get().unwrap();

        self.render(&mesh.vertex_buffer, &mesh.index_buffer);
    }

    fn handle(&mut self, event: Event) {
        println!("Handling event");
        match event {
            Event::WindowResized => self.recreate_swapchain().expect("Failed to recreate swapchain"),
            Event::CloseRequested => {
                unsafe { self.ctx.device.device_wait_idle().expect("Failed to wait for device") }
            }
        };
    }
}
