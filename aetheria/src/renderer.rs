use ash::vk;
use assets::{Mesh, MeshRegistry, ShaderRegistry};
use bytemuck::cast_slice;
use egui::mutex::Mutex;
use egui::TexturesDelta;
use glam::{Vec4, Vec3, Quat};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::{
    ops::Deref,
    sync::Arc,
};
use vulkan::VertexInputBuilder;
use vulkan::{
    Buffer, Context, DrawOptions, Image, Pool,
    Renderpass, Set, SetLayout, SetLayoutBuilder, Shader, Shaders, Swapchain, Texture,
    graphics, compute
};
use winit::event_loop::EventLoop;
use winit::window::Window;
use tracing::info;

use crate::camera::Camera;
use crate::include_bytes_align_as;
use crate::time::Time;
use crate::transform::Transform;

/*pub struct RenderObjectBuilder<'a> {
    renderer: &'a mut Renderer,
    mesh_registry: &'a mut MeshRegistry,
    mesh: Option<Arc<Mesh>>,
    color: Option<Vec4>,
    transform: Option<Transform>
}

impl RenderObjectBuilder<'_> {
    pub fn set_mesh(&mut self, path: &str) -> Result<&mut Self, vk::Result> {
         self.mesh = Some(self.mesh_registry.load(self.renderer, path));
         Ok(self)
    }
    
    pub fn set_color(&mut self, color: Vec4) -> &mut Self {
        self.color = Some(color);
        self
    }

    pub fn set_transform(&mut self, transform: Transform) -> &mut Self {
        self.transform = Some(transform);
        self
    }

    pub fn build(&mut self) -> Result<RenderObject, vk::Result> {
        if self.mesh.is_none() {
            panic!("Tried to create RenderObject with no mesh");
        }

        let material = Arc::new(Material::new(self.renderer, self.color.unwrap_or_else(|| Vec4::new(1.0, 1.0, 1.0, 1.0)))?);
        let transform = self.transform.clone().unwrap_or(Transform::IDENTITY);
        let transform_gpu = TransformGPU::new(self.renderer, transform)?;

        Ok(RenderObject {
            mesh: self.mesh.clone().unwrap(),
            material,
            transform: Arc::new(Mutex::new(transform_gpu))
        })
    }
}

#[derive(Clone)]
pub struct RenderObject {
    mesh: Arc<Mesh>,
    material: Arc<Material>,
    pub transform: Arc<Mutex<TransformGPU>>
}

impl RenderObject {
    pub fn builder<'a>(renderer: &'a mut Renderer, mesh_registry: &'a mut MeshRegistry) -> RenderObjectBuilder<'a> {
        RenderObjectBuilder {
            renderer,
            mesh_registry,
            mesh: None,
            color: None,
            transform: None
        }
    }
}

pub trait Renderable {
    fn get_objects(&self) -> Vec<&RenderObject>;
}*/

const PIXEL_WIDTH: u32 = 480;
const PIXEL_HEIGHT: u32 = 270;

pub struct Renderer {
    pub(crate) ctx: Context,
    window: Arc<Window>,

    render_finished: vk::Semaphore,
    in_flight: vk::Fence,

    output_layout: SetLayout,
    output_pool: Pool,
    output_texture: Texture,
    output_set: Set,

    render_pipeline: compute::Pipeline
}

impl Renderer {
    pub fn new(
        mut ctx: Context,
        shader_registry: &mut ShaderRegistry,
        window: Arc<Window>,
        event_loop: &EventLoop<()>,
    ) -> Result<Self, vk::Result> {
        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let render_finished =
            unsafe { ctx.device.create_semaphore(&semaphore_info, None).unwrap() };
        let in_flight = unsafe { ctx.device.create_fence(&fence_info, None).unwrap() };

        let output_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::STORAGE_IMAGE)
            .build()?;
        let output_pool = Pool::new(ctx.device.clone(), output_layout, 1)?;
        let output_image = Image::new(&ctx, ctx.swapchain.extent.width, ctx.swapchain.extent.height, ctx.swapchain.format, vk::ImageUsageFlags::STORAGE)?;
        let output_texture = Texture::from_image(&ctx, output_image, vk::Filter::NEAREST, vk::Filter::NEAREST)?;
        let output_set = output_pool.allocate()?;

        let shader = shader_registry.load(&ctx.device, "compute.glsl");
        let render_pipeline = compute::Pipeline::new(&ctx.device, shader.clone(), &[output_layout.clone()])?; 

        let renderer = Self {
            ctx,
            window,
            render_finished,
            in_flight,
            output_layout,
            output_pool,
            output_texture,
            output_set,
            render_pipeline
        };

        Ok(renderer)
    }

    unsafe fn destroy_swapchain(&mut self) {
        self.ctx.device.device_wait_idle().unwrap();

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

        
        let mut output_textures = Vec::new();
        let mut output_sets = Vec::new();
        for image in self.ctx.swapchain.images.clone() {
            let texture = Texture::from_image(&self.ctx, image, vk::Filter::NEAREST, vk::Filter::NEAREST)?;
            let set = self.output_pool.allocate()?;
            set.update_texture(&self.ctx.device, 0, &texture, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            output_textures.push(texture);
            output_sets.push(set);
        };
        self.output_textures = output_textures;
        self.output_sets = output_sets;

        Ok(())
    }

    pub fn render(&mut self) {
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

            let cmd = self.command_pool
                .allocate()
                .unwrap()
                .begin()
                .unwrap()
                .bind_compute_pipeline(self.render_pipeline.clone())
                .bind_descriptor_set(0, &self.output_sets[image_index as usize])
                .end()
                .unwrap(); 

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
