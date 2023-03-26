use ash::vk;
use glam::{Mat4, Vec3};

use std::rc::Rc;
use std::{
    ops::Deref,
    time::{SystemTime, UNIX_EPOCH},
};
use bevy_ecs::world::World;
use vulkan::{
    command::TransitionLayoutOptions, Buffer, Context, DrawOptions, Image, Pipeline, Pool,
    Renderpass, Set, SetLayout, SetLayoutBuilder, Shader, Shaders, Swapchain, Texture,
};
use winit::window::Window;
use vulkan::command::BufferBuilder;

use crate::include_bytes_align_as;
use crate::mesh::{MeshRef, MeshRegistry, TextureRegistry, TransformRef, TransformRegistry};

pub struct Renderer<'a> {
    pub(crate) ctx: Context,
    window: Rc<Window>,

    renderpass: Renderpass,
    pub pipeline: Pipeline,
    shaders: Shaders,
    framebuffers: Vec<vk::Framebuffer>,

    render_finished: vk::Semaphore,
    in_flight: vk::Fence,

    transform_layout: SetLayout,
    pub transform_pool: Pool<'a>,

    texture_layout: SetLayout,
    pub texture_pool: Pool<'a>,

    depth_image: Image,
    depth_view: vk::ImageView,
}

impl Renderer<'_> {
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

        let mut transform_pool = Pool::new(&ctx.device, transform_layout.clone(), 1)?;
        let mut texture_pool = Pool::new(&ctx.device, texture_layout.clone(), 1)?;

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
            texture_layout,
            texture_pool,
            depth_image,
            depth_view,
        };

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

    pub unsafe fn render(&mut self, world: &mut World) {
        let presentation_result =
            self.ctx
                .render(world, self.in_flight, |ctx, world, image_available, image_index| {
                    ctx.command_pool.clear(&ctx.device);
                    let mut cmd = ctx
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
                        .bind_pipeline(&self.pipeline);

                    let mut query = world.query::<(&MeshRef, &TransformRef)>();
                    let mesh_registry = world.get_resource::<MeshRegistry>().unwrap();
                    let transform_registry = world.get_resource::<TransformRegistry>().unwrap();
                    let texture_registry = world.get_resource::<TextureRegistry>().unwrap();
                    for (&mesh, &transform) in query.iter(world) {
                        let mesh = mesh_registry.get(mesh).unwrap();
                        let texture = texture_registry.get(mesh.texture.unwrap()).unwrap();
                        let transform = transform_registry.get(transform).unwrap();

                        cmd = cmd.bind_descriptor_set(&self.pipeline, 0, &transform.set)
                            .bind_descriptor_set(&self.pipeline, 1, &texture.set)
                            .bind_index_buffer(&mesh.index_buffer)
                            .bind_vertex_buffer(&mesh.vertex_buffer)
                            .draw(DrawOptions {
                                vertex_count: (mesh.index_buffer.size / 4).try_into().unwrap(),
                                instance_count: 1,
                                ..Default::default()
                            })
                    }

                    let cmd = cmd.end_renderpass()
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

impl Deref for Renderer<'_> {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}
