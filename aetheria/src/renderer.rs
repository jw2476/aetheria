use ash::vk;
use bevy_ecs::prelude::Component;
use bevy_ecs::system::{Res, ResMut, Resource};
use glam::{Mat4, Vec3};

use bevy_ecs::{system::Query, world::World};
use std::ops::DerefMut;
use std::rc::Rc;
use std::{
    ops::Deref,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use vulkan::command::BufferBuilder;
use vulkan::{
    command::TransitionLayoutOptions, Buffer, Context, DrawOptions, Image, Pipeline, Pool,
    Renderpass, Set, SetLayout, SetLayoutBuilder, Shader, Shaders, Swapchain, Texture,
};
use winit::window::Window;

use crate::camera::Camera;
use crate::include_bytes_align_as;
use crate::mesh::{
    MaterialRef, MaterialRegistry, MeshRef, MeshRegistry, TextureRegistry, TransformRef,
    TransformRegistry,
};

#[derive(Resource)]
pub struct Renderer {
    pub(crate) ctx: Context,
    window: Arc<Window>,

    renderpass: Renderpass,
    pub pipeline: Pipeline,
    shaders: Shaders,
    framebuffers: Vec<vk::Framebuffer>,

    render_finished: vk::Semaphore,
    in_flight: vk::Fence,

    camera_layout: SetLayout,
    pub camera_pool: Pool,

    material_layout: SetLayout,
    pub material_pool: Pool,

    transform_layout: SetLayout,
    pub transform_pool: Pool,

    depth_image: Image,
    depth_view: vk::ImageView,
}

impl Renderer {
    pub fn new(ctx: Context, window: Arc<Window>) -> Result<Self, vk::Result> {
        let camera_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::UNIFORM_BUFFER)
            .build()?;
        let material_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::UNIFORM_BUFFER)
            .add(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .build()?;
        let transform_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::UNIFORM_BUFFER)
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

        let descriptor_layouts = &[
            camera_layout.clone(),
            material_layout.clone(),
            transform_layout.clone(),
        ];
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

        let camera_pool = Pool::new(ctx.device.clone(), camera_layout.clone(), 1000).unwrap();
        let material_pool = Pool::new(ctx.device.clone(), material_layout.clone(), 1000).unwrap();
        let transform_pool = Pool::new(ctx.device.clone(), transform_layout.clone(), 1000).unwrap();

        let mut renderer = Self {
            ctx,
            window,
            renderpass,
            pipeline,
            shaders,
            framebuffers,
            render_finished,
            in_flight,
            depth_image,
            depth_view,
            camera_layout,
            material_layout,
            transform_layout,
            camera_pool,
            material_pool,
            transform_pool,
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

        let descriptor_layouts = &[
            self.camera_layout.clone(),
            self.material_layout.clone(),
            self.transform_layout.clone(),
        ];
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

    pub fn render(
        mut renderer: ResMut<Self>,
        mesh_registry: Res<MeshRegistry>,
        transform_registry: Res<TransformRegistry>,
        material_registry: Res<MaterialRegistry>,
        camera: Res<Camera>,
        query: Query<(&MeshRef, &TransformRef)>,
    ) {
        unsafe {
            let in_flight = renderer.in_flight.clone();

            let acquire_result = renderer.ctx.start_frame(in_flight);

            let image_index = match acquire_result {
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    renderer
                        .recreate_swapchain()
                        .expect("Swapchain recreation failed");
                    return;
                }
                Err(e) => panic!("{}", e),
                Ok(image_index) => image_index,
            };

            renderer.ctx.command_pool.clear();
            let mut cmd = renderer
                .ctx
                .command_pool
                .allocate()
                .unwrap()
                .begin()
                .unwrap()
                .begin_renderpass(
                    &renderer.renderpass,
                    renderer.framebuffers[image_index as usize],
                    renderer.ctx.swapchain.extent,
                )
                .bind_pipeline(&renderer.pipeline)
                .bind_descriptor_set(&renderer.pipeline, 0, &camera.set);

            for (&mesh, &transform) in query.iter() {
                let mesh = mesh_registry.get(mesh).unwrap();
                let material = material_registry.get(mesh.material.unwrap()).unwrap();
                let transform = transform_registry.get(transform).unwrap();

                cmd = cmd
                    .bind_descriptor_set(&renderer.pipeline, 1, &material.set)
                    .bind_descriptor_set(&renderer.pipeline, 2, &transform.set)
                    .bind_index_buffer(&mesh.index_buffer)
                    .bind_vertex_buffer(&mesh.vertex_buffer)
                    .draw(DrawOptions {
                        vertex_count: (mesh.index_buffer.size / 4).try_into().unwrap(),
                        instance_count: 1,
                        ..Default::default()
                    })
            }

            let cmd = cmd.end_renderpass().end().unwrap();

            let wait_semaphores = &[renderer.ctx.image_available];
            let signal_semaphores = &[renderer.render_finished];
            let command_buffers = &[*cmd];
            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(wait_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .command_buffers(command_buffers)
                .signal_semaphores(signal_semaphores);

            renderer
                .ctx
                .device
                .queue_submit(
                    renderer.ctx.device.queues.graphics.queue,
                    &[*submit_info],
                    renderer.in_flight,
                )
                .unwrap();

            let presentation_result = renderer
                .ctx
                .end_frame(image_index, renderer.render_finished);

            match presentation_result {
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => renderer
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

impl DerefMut for Renderer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}
