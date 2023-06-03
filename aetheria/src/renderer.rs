use ash::vk;
use assets::{Mesh, MeshRegistry};
use bytemuck::cast_slice;
use egui::mutex::Mutex;
use egui::TexturesDelta;
use glam::Vec4;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::{
    ops::Deref,
    sync::Arc,
};
use vulkan::VertexInputBuilder;
use vulkan::{
    Buffer, Context, DrawOptions, Image, Pipeline, Pool,
    Renderpass, Set, SetLayout, SetLayoutBuilder, Shader, Shaders, Swapchain, Texture,
};
use winit::event_loop::EventLoop;
use winit::window::Window;
use tracing::info;

use crate::camera::Camera;
use crate::include_bytes_align_as;
use crate::material::Material;
use crate::mesh::EguiTexture;

pub struct Renderer {
    pub(crate) ctx: Context,
    window: Arc<Window>,

    render_renderpass: Renderpass,
    render_framebuffer: vk::Framebuffer,
    upscale_renderpass: Renderpass,
    upscale_framebuffers: Vec<vk::Framebuffer>,

    render_finished: vk::Semaphore,
    in_flight: vk::Fence,

    render_layout: SetLayout,
    pub render_pool: Pool,
    render_set: Set,

    camera_layout: SetLayout,
    pub camera_pool: Pool,

    material_layout: SetLayout,
    pub material_pool: Pool,

    transform_layout: SetLayout,
    pub transform_pool: Pool,

    render_texture: Texture,
    render_view: vk::ImageView,
    depth_image: Image,
    depth_view: vk::ImageView,

    pub egui_ctx: egui::Context,
    pub egui_winit_state: Arc<Mutex<egui_winit::State>>,
    egui_textures: HashMap<egui::TextureId, EguiTexture>,

    render_shaders: Shaders,
    render_pipeline: Pipeline,
    upscale_shaders: Shaders,
    upscale_pipeline: Pipeline,
    ui_shaders: Shaders,
    ui_pipeline: Pipeline,

    egui_texture_layout: SetLayout,
    pub egui_texture_pool: Pool,
}

#[derive(Clone)]
pub struct MeshMaterial(Arc<Mesh>, Arc<Material>);
impl MeshMaterial {
    pub fn new(renderer: &mut Renderer, mesh_registry: &mut MeshRegistry, mesh: &str, color: Vec4) -> Result<MeshMaterial, vk::Result> {
        let mesh = mesh_registry.load(&renderer.ctx, mesh);
        let material = Arc::new(Material::new(renderer, color)?);
        Ok(Self(mesh, material))
    }
}

const PIXEL_WIDTH: u32 = 480;
const PIXEL_HEIGHT: u32 = 270;

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
            .build()?;
        let transform_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::UNIFORM_BUFFER)
            .build()?;
        let render_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .build()?;
        let egui_texture_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .build()?;

        let depth_image = Image::new(
            &ctx,
            PIXEL_WIDTH,
            PIXEL_HEIGHT,
            vk::Format::D32_SFLOAT,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        )?;
        let depth_view = depth_image.create_view(&ctx)?;

        let render_image = Image::new(
            &ctx,
            PIXEL_WIDTH,
            PIXEL_HEIGHT,
            ctx.swapchain.format,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED  
        )?;
        let render_view = render_image.create_view(&ctx)?;
        let render_texture = Texture::from_image(&ctx, render_image, vk::Filter::NEAREST, vk::Filter::NEAREST)?;

        let render_renderpass = Renderpass::new_render(&ctx.device, ctx.swapchain.format)?;
        let upscale_renderpass = Renderpass::new_upscale_ui(&ctx.device, ctx.swapchain.format)?;

        let render_vertex_shader = Shader::new(
            &ctx.device,
            include_bytes_align_as!(u32, "../../assets/shaders/compiled/render.vert.spv"),
            vk::ShaderStageFlags::VERTEX,
        )?;
        let render_fragment_shader = Shader::new(
            &ctx.device,
            include_bytes_align_as!(u32, "../../assets/shaders/compiled/render.frag.spv"),
            vk::ShaderStageFlags::FRAGMENT,
        )?;
        let render_shaders = Shaders {
            vertex: Some(render_vertex_shader),
            fragment: Some(render_fragment_shader),
        };

        let render_descriptor_layouts = &[
            camera_layout.clone(),
            material_layout.clone()
        ];

        let render_vertex_input = VertexInputBuilder::new().add_binding(|binding| {
            binding
                .add_attribute(vk::Format::R32G32B32_SFLOAT)
                .add_attribute(vk::Format::R32G32_SFLOAT)
                .add_attribute(vk::Format::R32G32B32_SFLOAT)
        });

        let render_pipeline = Pipeline::new(
            &ctx.device,
            &render_renderpass,
            render_shaders.clone(),
            vk::Extent2D { width: PIXEL_WIDTH, height: PIXEL_HEIGHT },
            render_descriptor_layouts,
            render_vertex_input,
            0,
            true,
            true,
        )?;
        
        let render_framebuffer = render_renderpass.create_framebuffer(&ctx.device, PIXEL_WIDTH, PIXEL_HEIGHT, &[render_view, depth_view])?;

        let upscale_vertex_shader = Shader::new(
            &ctx.device,
            include_bytes_align_as!(u32, "../../assets/shaders/compiled/upscale.vert.spv"),
            vk::ShaderStageFlags::VERTEX,
        )?;
        let upscale_fragment_shader = Shader::new(
            &ctx.device,
            include_bytes_align_as!(u32, "../../assets/shaders/compiled/upscale.frag.spv"),
            vk::ShaderStageFlags::FRAGMENT,
        )?;
        let upscale_shaders = Shaders {
            vertex: Some(upscale_vertex_shader),
            fragment: Some(upscale_fragment_shader),
        };

        let upscale_descriptor_layouts = &[render_layout.clone()];
        let upscale_vertex_input = VertexInputBuilder::new();

        let upscale_pipeline = Pipeline::new(
            &ctx.device,
            &upscale_renderpass,
            upscale_shaders.clone(),
            ctx.swapchain.extent,
            upscale_descriptor_layouts,
            upscale_vertex_input,
            0,
            false,
            false,
        )?;

        let upscale_framebuffers: Vec<vk::Framebuffer> =
            std::iter::zip(&ctx.swapchain.images, &ctx.swapchain.image_views)
                .map(|(image, &view)| {
                    upscale_renderpass
                        .create_framebuffer(
                            &ctx.device,
                            image.width,
                            image.height,
                            &[view],
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
        let mut render_pool = Pool::new(ctx.device.clone(), render_layout.clone(), 1000).unwrap();
        let render_set = render_pool.allocate()?;
        render_set.update_texture(&ctx.device, 0, &render_texture, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

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
            &upscale_renderpass,
            ui_shaders.clone(),
            ctx.swapchain.extent,
            ui_descriptor_layouts,
            ui_vertex_input,
            1,
            false,
            false,
        )?;

        let renderer = Self {
            ctx,
            window,
            render_renderpass,
            upscale_renderpass,
            render_pipeline,
            upscale_pipeline,
            render_shaders,
            upscale_shaders,
            render_framebuffer,
            upscale_framebuffers,
            render_finished,
            in_flight,
            render_view,
            render_texture,
            depth_image,
            depth_view,
            camera_layout,
            material_layout,
            transform_layout,
            render_layout,
            camera_pool,
            material_pool,
            transform_pool,
            render_pool,
            render_set,
            egui_ctx,
            egui_winit_state,
            egui_textures: HashMap::new(),
            ui_shaders,
            ui_pipeline,
            egui_texture_layout,
            egui_texture_pool,
        };

        Ok(renderer)
    }

    unsafe fn destroy_swapchain(&mut self) {
        self.ctx.device.device_wait_idle().unwrap();

        self.upscale_framebuffers
            .iter()
            .for_each(|framebuffer| self.ctx.device.destroy_framebuffer(*framebuffer, None));
        self.ctx.device.destroy_pipeline(*self.upscale_pipeline, None);
        self.ctx
            .device
            .destroy_pipeline_layout(self.upscale_pipeline.layout, None);
        self.ctx.device.destroy_render_pass(*self.upscale_renderpass, None);
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

        info!("Recreating swapchain");

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

        self.upscale_renderpass = Renderpass::new_upscale_ui(
            &self.ctx.device,
            self.ctx.swapchain.format,
        )?;

        let descriptor_layouts = &[self.render_layout.clone()];

        let vertex_input = VertexInputBuilder::new().add_binding(|binding| {
            binding
                .add_attribute(vk::Format::R32G32B32_SFLOAT)
                .add_attribute(vk::Format::R32G32_SFLOAT)
                .add_attribute(vk::Format::R32G32B32_SFLOAT)
        });

        self.upscale_pipeline = Pipeline::new(
            &self.ctx.device,
            &self.upscale_renderpass,
            self.upscale_shaders.clone(),
            self.ctx.swapchain.extent,
            descriptor_layouts,
            vertex_input,
            0,
            false,
            false,
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
            &self.upscale_renderpass,
            self.ui_shaders.clone(),
            self.ctx.swapchain.extent,
            ui_descriptor_layouts,
            ui_vertex_input,
            1,
            false,
            false,
        )?;

        self.upscale_framebuffers =
            std::iter::zip(&self.ctx.swapchain.images, &self.ctx.swapchain.image_views)
                .map(|(image, &view)| {
                    self.upscale_renderpass
                        .create_framebuffer(
                            &self.ctx.device,
                            image.width,
                            image.height,
                            &[view],
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
    ) {
        textures_delta.set.iter().for_each(|(texture_id, delta)| {
            println!("{:?}", texture_id);
            match self.egui_textures.get(texture_id) {
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
                    self.egui_textures.insert(*texture_id, texture);
                }
            }
        });
    }

    pub fn render(&mut self, objects: &[MeshMaterial], camera: &Camera) {
        unsafe {
            let in_flight = self.in_flight.clone();

            let acquire_result = self.ctx.start_frame(in_flight);

            let image_index = match acquire_result {
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self
                        .recreate_swapchain()
                        .expect("Swapchain recreation failed");
                    return;
                }
                Err(e) => panic!("{}", e),
                Ok(image_index) => image_index,
            };

            let input = self
                .egui_winit_state
                .lock()
                .take_egui_input(&self.window)
                .clone();
            self.egui_ctx.begin_frame(input);
            egui::Window::new("Entity Editor")
                .resizable(true)
                .show(&self.egui_ctx, |ui| {
                    ui.allocate_space(ui.available_size());
                });
            let full_output = self.egui_ctx.end_frame();

            self.egui_winit_state.lock().handle_platform_output(
                &self.window,
                &self.egui_ctx,
                full_output.platform_output,
            );

            self.handle_egui_textures(full_output.textures_delta);

            let primitives = self.egui_ctx.tessellate(full_output.shapes);
            let pixels_per_point = self.egui_ctx.pixels_per_point();
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
                                    self.swapchain.extent.width as f32 / pixels_per_point,
                                    self.swapchain.extent.height as f32 / pixels_per_point,
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
                            &self.ctx,
                            vertices,
                            vk::BufferUsageFlags::VERTEX_BUFFER,
                        )
                        .unwrap();
                        let index_buffer =
                            Buffer::new(&self.ctx, indices, vk::BufferUsageFlags::INDEX_BUFFER)
                                .unwrap();

                        let set = self.egui_textures
                            .get(&mesh.texture_id.into())
                            .unwrap()
                            .set
                            .clone();

                        (vertex_buffer, index_buffer, set)
                    }
                    epaint::Primitive::Callback(_) => todo!(),
                })
                .collect::<Vec<(Buffer, Buffer, Set)>>();

            self.ctx.command_pool.clear();
            let mut cmd = self
                .ctx
                .command_pool
                .allocate()
                .unwrap()
                .begin()
                .unwrap()
                .begin_renderpass(&self.render_renderpass, self.render_framebuffer, vk::Extent2D { width: PIXEL_WIDTH, height: PIXEL_HEIGHT })
                .bind_pipeline(&self.render_pipeline)
                .bind_descriptor_set(&self.render_pipeline, 0, &camera.set);


            for MeshMaterial(mesh, material) in objects {
                cmd = cmd
                    .bind_index_buffer(&mesh.index_buffer)
                    .bind_vertex_buffer(&mesh.vertex_buffer)
                    .bind_descriptor_set(&self.render_pipeline, 1, &material.set)
                    .draw(DrawOptions {
                        vertex_count: (mesh.index_buffer.size / 4).try_into().unwrap(),
                        instance_count: 1,
                        ..Default::default()
                    })
            }

            cmd = cmd.end_renderpass()
                .begin_renderpass(&self.upscale_renderpass, self.upscale_framebuffers[image_index as usize], self.ctx.swapchain.extent)
                .bind_pipeline(&self.upscale_pipeline)
                .bind_descriptor_set(&self.upscale_pipeline, 0, &self.render_set)
                .draw(DrawOptions { vertex_count: 3, instance_count: 1, first_vertex: 0, first_instance: 0 })
                .next_subpass()
                .bind_pipeline(&self.ui_pipeline);
            for (vertex_buffer, index_buffer, set) in &buffers {
                cmd = cmd
                    .bind_descriptor_set(&self.ui_pipeline, 0, &set)
                    .bind_vertex_buffer(vertex_buffer)
                    .bind_index_buffer(index_buffer)
                    .draw(DrawOptions {
                        vertex_count: (index_buffer.size / 4).try_into().unwrap(),
                        instance_count: 1,
                        ..Default::default()
                    });
            }

            let cmd = cmd.end_renderpass().end().unwrap();

            let wait_semaphores = &[self.ctx.image_available];
            let signal_semaphores = &[self.render_finished];
            let command_buffers = &[*cmd];
            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(wait_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .command_buffers(command_buffers)
                .signal_semaphores(signal_semaphores);

            self
                .ctx
                .device
                .queue_submit(
                    self.ctx.device.queues.graphics.queue,
                    &[*submit_info],
                    self.in_flight,
                )
                .unwrap();

            let presentation_result = self
                .ctx
                .end_frame(image_index, self.render_finished);

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

impl DerefMut for Renderer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}
