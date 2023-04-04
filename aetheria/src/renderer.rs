use ash::vk;
use bevy_ecs::prelude::Component;
use bevy_ecs::system::{Res, ResMut, Resource};
use bytemuck::cast_slice;
use egui::mutex::Mutex;
use egui::TexturesDelta;
use glam::{Mat4, Vec2, Vec3};

use bevy_ecs::{system::Query, world::World};
use std::ops::DerefMut;
use std::rc::Rc;
use std::{
    ops::Deref,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use vulkan::command::BufferBuilder;
use vulkan::VertexInputBuilder;
use vulkan::{
    command::TransitionLayoutOptions, Buffer, Context, DrawOptions, Image, Pipeline, Pool,
    Renderpass, Set, SetLayout, SetLayoutBuilder, Shader, Shaders, Swapchain, Texture,
};
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::camera::Camera;
use crate::include_bytes_align_as;
use crate::mesh::{
    EguiTexture, EguiTextureRef, EguiTextureRegistry, MaterialRef, MaterialRegistry, MeshRef,
    MeshRegistry, TextureRegistry, TransformRef, TransformRegistry,
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

    pub egui_ctx: egui::Context,
    pub egui_winit_state: Arc<Mutex<egui_winit::State>>,

    ui_shaders: Shaders,
    ui_pipeline: Pipeline,

    egui_texture_layout: SetLayout,
    pub egui_texture_pool: Pool,
}

impl Renderer {
    pub fn new(
        ctx: Context,
        window: Arc<Window>,
        event_loop: &EventLoop<()>,
    ) -> Result<Self, vk::Result> {
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
        let egui_texture_layout = SetLayoutBuilder::new(&ctx.device)
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

        let descriptor_layouts = &[
            camera_layout.clone(),
            material_layout.clone(),
            transform_layout.clone(),
        ];

        let vertex_input = VertexInputBuilder::new().add_binding(|binding| {
            binding
                .add_attribute(vk::Format::R32G32B32_SFLOAT)
                .add_attribute(vk::Format::R32G32_SFLOAT)
                .add_attribute(vk::Format::R32G32B32_SFLOAT)
        });

        let pipeline = Pipeline::new(
            &ctx.device,
            &renderpass,
            shaders.clone(),
            ctx.swapchain.extent,
            descriptor_layouts,
            vertex_input,
            0,
            true,
            true,
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
        let egui_texture_pool =
            Pool::new(ctx.device.clone(), egui_texture_layout.clone(), 1000).unwrap();

        let egui_ctx = egui::Context::default();
        let egui_winit_state = Arc::new(Mutex::new(egui_winit::State::new(event_loop)));

        let vertex_shader = Shader::new(
            &ctx.device,
            include_bytes_align_as!(u32, "../../assets/shaders/compiled/ui.vert.spv"),
            vk::ShaderStageFlags::VERTEX,
        )?;
        let fragment_shader = Shader::new(
            &ctx.device,
            include_bytes_align_as!(u32, "../../assets/shaders/compiled/ui.frag.spv"),
            vk::ShaderStageFlags::FRAGMENT,
        )?;
        let ui_shaders = Shaders {
            vertex: Some(vertex_shader),
            fragment: Some(fragment_shader),
        };

        let ui_vertex_input = VertexInputBuilder::new().add_binding(|binding| {
            binding
                .add_attribute(vk::Format::R32G32_SFLOAT)
                .add_attribute(vk::Format::R32G32B32A32_SFLOAT)
                .add_attribute(vk::Format::R32G32_SFLOAT)
        });

        let ui_descriptor_layouts = &[egui_texture_layout.clone()];
        let ui_pipeline = Pipeline::new(
            &ctx.device,
            &renderpass,
            ui_shaders.clone(),
            ctx.swapchain.extent,
            ui_descriptor_layouts,
            ui_vertex_input,
            1,
            false,
            false,
        )?;

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
            egui_ctx,
            egui_winit_state,
            ui_shaders,
            ui_pipeline,
            egui_texture_layout,
            egui_texture_pool,
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

        let vertex_input = VertexInputBuilder::new().add_binding(|binding| {
            binding
                .add_attribute(vk::Format::R32G32B32_SFLOAT)
                .add_attribute(vk::Format::R32G32_SFLOAT)
                .add_attribute(vk::Format::R32G32B32_SFLOAT)
        });

        self.pipeline = Pipeline::new(
            &self.ctx.device,
            &self.renderpass,
            self.shaders.clone(),
            self.ctx.swapchain.extent,
            descriptor_layouts,
            vertex_input,
            0,
            true,
            true,
        )?;

        let ui_vertex_input = VertexInputBuilder::new().add_binding(|binding| {
            binding
                .add_attribute(vk::Format::R32G32_SFLOAT)
                .add_attribute(vk::Format::R32G32B32A32_SFLOAT)
                .add_attribute(vk::Format::R32G32_SFLOAT)
        });

        let ui_descriptor_layouts = &[self.egui_texture_layout.clone()];
        self.ui_pipeline = Pipeline::new(
            &self.ctx.device,
            &self.renderpass,
            self.ui_shaders.clone(),
            self.ctx.swapchain.extent,
            ui_descriptor_layouts,
            ui_vertex_input,
            1,
            false,
            false,
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

    fn color_32_to_rgba(color: epaint::Color32) -> Vec<f32> {
        let color = color
            .to_array()
            .iter()
            .map(|channel| (*channel as f32) / 255.0)
            .collect::<Vec<f32>>();

        color
    }

    fn handle_egui_textures(
        &mut self,
        textures_delta: TexturesDelta,
        egui_texture_registry: &mut EguiTextureRegistry,
    ) {
        textures_delta.set.iter().for_each(|(texture_id, delta)| {
            println!("{:?}", texture_id);
            match egui_texture_registry.get(&(*texture_id).into()) {
                Some(_) => {
                    println!("Update Texture: {:?}", delta.pos);
                    todo!();
                }
                None => {
                    println!("New Texture: {:?}", delta.pos);
                    let (width, height, pixels) = match &delta.image {
                        epaint::image::ImageData::Color(image) => {
                            let pixels = image
                                .pixels
                                .iter()
                                .cloned()
                                .flat_map(|color| color.to_array())
                                .collect::<Vec<u8>>();

                            (
                                image.width().try_into().unwrap(),
                                image.height().try_into().unwrap(),
                                pixels,
                            )
                        }
                        epaint::image::ImageData::Font(image) => {
                            let pixels = image
                                .srgba_pixels(None)
                                .flat_map(|color| color.to_array())
                                .collect::<Vec<u8>>();

                            (
                                image.width().try_into().unwrap(),
                                image.height().try_into().unwrap(),
                                pixels,
                            )
                        }
                    };

                    let texture = EguiTexture::new(self, &pixels, width, height).unwrap();
                    egui_texture_registry.add(texture);
                }
            }
        });
    }

    pub fn render(
        mut renderer: ResMut<Self>,
        mesh_registry: Res<MeshRegistry>,
        mut transform_registry: ResMut<TransformRegistry>,
        material_registry: Res<MaterialRegistry>,
        mut egui_texture_registry: ResMut<EguiTextureRegistry>,
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

            let input = renderer
                .egui_winit_state
                .lock()
                .take_egui_input(&renderer.window)
                .clone();
            renderer.egui_ctx.begin_frame(input);
            egui::Window::new("Entity Editor")
                .resizable(true)
                .show(&renderer.egui_ctx, |ui| {
                    query.iter().for_each(|(_, transform_ref)| {
                        let transform = transform_registry.get_mut(transform_ref).unwrap();
                        ui.add(egui::DragValue::new(&mut transform.translation.x).speed(0.1));
                    });

                    ui.allocate_space(ui.available_size());
                });
            let full_output = renderer.egui_ctx.end_frame();

            renderer.egui_winit_state.lock().handle_platform_output(
                &renderer.window,
                &renderer.egui_ctx,
                full_output.platform_output,
            );

            renderer.handle_egui_textures(full_output.textures_delta, &mut egui_texture_registry);

            let primitives = renderer.egui_ctx.tessellate(full_output.shapes);
            let pixels_per_point = renderer.egui_ctx.pixels_per_point();
            let buffers = primitives
                .iter()
                .map(|primitive| match &primitive.primitive {
                    epaint::Primitive::Mesh(mesh) => {
                        let vertices = mesh
                            .vertices
                            .iter()
                            .flat_map(|vertex| {
                                let mut bytes = Vec::new();
                                let points = epaint::Vec2::new(
                                    renderer.swapchain.extent.width as f32 / pixels_per_point,
                                    renderer.swapchain.extent.height as f32 / pixels_per_point,
                                );

                                let zero_to_one = vertex.pos.to_vec2() / points;
                                let minus_one_to_one =
                                    (zero_to_one * 2.0) - epaint::Vec2::new(1.0, 1.0);
                                bytes.append(
                                    &mut cast_slice::<epaint::Vec2, u8>(&[minus_one_to_one])
                                        .to_vec(),
                                );

                                let color = Self::color_32_to_rgba(vertex.color);
                                bytes.append(&mut cast_slice::<f32, u8>(&color).to_vec());

                                bytes.append(
                                    &mut cast_slice::<epaint::Pos2, u8>(&[vertex.uv]).to_vec(),
                                );

                                bytes
                            })
                            .collect::<Vec<u8>>();
                        let indices = cast_slice::<u32, u8>(&mesh.indices);

                        let vertex_buffer = Buffer::new(
                            &renderer.ctx,
                            vertices,
                            vk::BufferUsageFlags::VERTEX_BUFFER,
                        )
                        .unwrap();
                        let index_buffer =
                            Buffer::new(&renderer.ctx, indices, vk::BufferUsageFlags::INDEX_BUFFER)
                                .unwrap();

                        let set = egui_texture_registry
                            .get(&mesh.texture_id.into())
                            .unwrap()
                            .set
                            .clone();

                        (vertex_buffer, index_buffer, set)
                    }
                    epaint::Primitive::Callback(_) => todo!(),
                })
                .collect::<Vec<(Buffer, Buffer, Set)>>();

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
                let mesh = mesh_registry.get(&mesh).unwrap();
                let material = material_registry.get(&mesh.material.unwrap()).unwrap();
                let transform = transform_registry.get(&transform).unwrap();

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

            cmd = cmd.next_subpass().bind_pipeline(&renderer.ui_pipeline);
            for (vertex_buffer, index_buffer, set) in &buffers {
                cmd = cmd
                    .bind_descriptor_set(&renderer.ui_pipeline, 0, &set)
                    .bind_vertex_buffer(vertex_buffer)
                    .bind_index_buffer(index_buffer)
                    .draw(DrawOptions {
                        vertex_count: (index_buffer.size / 4).try_into().unwrap(),
                        instance_count: 1,
                        ..Default::default()
                    });
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
