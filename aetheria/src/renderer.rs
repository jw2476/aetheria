use ash::vk;
use assets::{ShaderRegistry, Vertex, Mesh};
use bytemuck::{cast_slice, Zeroable, Pod, cast_slice_mut};
use egui::mutex::Mutex;
use egui::TexturesDelta;
use glam::{Vec4, Vec3, Quat};
use vulkan::command::TransitionLayoutOptions;
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

    per_frame_layout: SetLayout,
    per_frame_pool: Pool,
    per_frame_set: Set,
    output_texture: Texture,
    camera_buffer: Buffer,
    time_buffer: Buffer,
    
    geometry_layout: SetLayout,
    geometry_pool: Pool,
    geometry_set: Set,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    mesh_buffer: Buffer,
    material_buffer: Buffer,

    render_pipeline: compute::Pipeline
}

const RENDER_WIDTH: u32 = 480;
const RENDER_HEIGHT: u32 = 270;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct MeshData {
    first_index: i32,
    num_indices: i32,
    material: i32
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct Material {
    albedo: Vec3,
    emission: f32,
    roughness: f32,
    metalness: f32,
    _padding: [f32; 2]
}

impl Renderer {
    pub fn new(
        ctx: Context,
        shader_registry: &mut ShaderRegistry,
        window: Arc<Window>,
        mesh: &Mesh
    ) -> Result<Self, vk::Result> {
        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let render_finished =
            unsafe { ctx.device.create_semaphore(&semaphore_info, None).unwrap() };
        let in_flight = unsafe { ctx.device.create_fence(&fence_info, None).unwrap() };

        let per_frame_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::STORAGE_IMAGE)
            .add(vk::DescriptorType::UNIFORM_BUFFER)
            .add(vk::DescriptorType::UNIFORM_BUFFER)
            .build()?;
        let mut per_frame_pool = Pool::new(ctx.device.clone(), per_frame_layout.clone(), 1)?;
        let output_image = Image::new(&ctx, RENDER_WIDTH, RENDER_HEIGHT, vk::Format::R8G8B8A8_UNORM, vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC)?;
        let output_texture = Texture::from_image(&ctx, output_image, vk::Filter::NEAREST, vk::Filter::NEAREST)?;
        let camera_buffer = Buffer::new(&ctx, vec![0_u8; 32], vk::BufferUsageFlags::UNIFORM_BUFFER)?;
        let time_buffer = Buffer::new(&ctx, vec![0_u8; 8], vk::BufferUsageFlags::UNIFORM_BUFFER)?;
        let per_frame_set = per_frame_pool.allocate()?;
        per_frame_set.update_texture(&ctx.device, 0, &output_texture, vk::ImageLayout::GENERAL);
        per_frame_set.update_buffer(&ctx.device, 1, &camera_buffer);
        per_frame_set.update_buffer(&ctx.device, 2, &time_buffer);

        let geometry_layout = SetLayoutBuilder::new(&ctx.device)
                .add(vk::DescriptorType::STORAGE_BUFFER)
                .add(vk::DescriptorType::STORAGE_BUFFER)
                .add(vk::DescriptorType::STORAGE_BUFFER)
                .add(vk::DescriptorType::STORAGE_BUFFER)
                .build()?;
        let mut geometry_pool = Pool::new(ctx.device.clone(), geometry_layout.clone(), 1)?;
        let geometry_set = geometry_pool.allocate()?;

        let vertex_buffer = Buffer::new(&ctx, cast_slice::<Vertex, u8>(&mesh.vertices), vk::BufferUsageFlags::STORAGE_BUFFER)?;
        let index_buffer = Buffer::new(&ctx, cast_slice::<u32, u8>(&mesh.indices), vk::BufferUsageFlags::STORAGE_BUFFER)?;

        let glowy_mesh = MeshData { first_index: 0, num_indices: mesh.indices.len() as i32, material: 0 };
        let green_mesh = MeshData { first_index: 0, num_indices: mesh.indices.len() as i32, material: 1 };
        let mut mesh_bytes = cast_slice::<MeshData, u8>(&[glowy_mesh, green_mesh]).to_vec();
        let mut mesh_buffer = cast_slice::<i32, u8>(&[2]).to_vec();
        mesh_buffer.append(&mut mesh_bytes);
        let mesh_buffer = Buffer::new(&ctx, mesh_buffer, vk::BufferUsageFlags::STORAGE_BUFFER)?;

        let glowy_material = Material { albedo: Vec3::new(1.0, 1.0, 1.0), emission: 1.0, roughness: 1.0, metalness: 0.0, ..Default::default() };
        let green_material = Material { albedo: Vec3::new(0.0, 1.0, 0.0), emission: 0.0, roughness: 1.0, metalness: 0.0, ..Default::default() };
        let material_buffer = Buffer::new(&ctx, cast_slice::<Material, u8>(&[glowy_material, green_material]), vk::BufferUsageFlags::STORAGE_BUFFER)?;

        geometry_set.update_buffer(&ctx.device, 0, &vertex_buffer);
        geometry_set.update_buffer(&ctx.device, 1, &index_buffer);
        geometry_set.update_buffer(&ctx.device, 2, &mesh_buffer);
        geometry_set.update_buffer(&ctx.device, 3, &material_buffer);

        let shader = shader_registry.load(&ctx.device, "test.comp.glsl");
        let render_pipeline = compute::Pipeline::new(&ctx.device, shader.clone(), &[per_frame_layout.clone(), geometry_layout.clone()])?; 

        let renderer = Self {
            ctx,
            window,
            render_finished,
            in_flight,
            per_frame_layout,
            per_frame_pool,
            geometry_layout,
            geometry_pool,
            geometry_set,
            vertex_buffer,
            index_buffer,
            mesh_buffer,
            material_buffer,
            output_texture,
            camera_buffer,
            time_buffer,
            per_frame_set,
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

        Ok(())
    }

    pub fn render(&mut self, camera: &Camera, time: &Time) {
        camera.update_buffer(&mut self.camera_buffer); 
        time.update_buffer(&mut self.time_buffer);

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

            self.command_pool.clear();

            let cmd = self.command_pool
                .allocate()
                .unwrap()
                .begin()
                .unwrap()
                .bind_compute_pipeline(self.render_pipeline.clone())
                .bind_descriptor_set(0, &self.per_frame_set)
                .bind_descriptor_set(1, &self.geometry_set)
                .transition_image_layout(&self.output_texture.image, &TransitionLayoutOptions { 
                    old: vk::ImageLayout::UNDEFINED, 
                    new: vk::ImageLayout::GENERAL, 
                    source_access: vk::AccessFlags::NONE, 
                    destination_access: vk::AccessFlags::SHADER_WRITE, 
                    source_stage: vk::PipelineStageFlags::TOP_OF_PIPE, 
                    destination_stage: vk::PipelineStageFlags::COMPUTE_SHADER 
                })
                .transition_image_layout(&self.ctx.swapchain.images[image_index as usize], &TransitionLayoutOptions { 
                    old: vk::ImageLayout::UNDEFINED, 
                    new: vk::ImageLayout::TRANSFER_DST_OPTIMAL, 
                    source_access: vk::AccessFlags::NONE, 
                    destination_access: vk::AccessFlags::TRANSFER_WRITE, 
                    source_stage: vk::PipelineStageFlags::TOP_OF_PIPE, 
                    destination_stage: vk::PipelineStageFlags::TRANSFER 
                })
                .dispatch(RENDER_WIDTH / 16, (RENDER_HEIGHT as f32 / 16.0).ceil() as u32, 1)
                .transition_image_layout(&self.output_texture.image, &TransitionLayoutOptions { 
                    old: vk::ImageLayout::GENERAL, 
                    new: vk::ImageLayout::TRANSFER_SRC_OPTIMAL, 
                    source_access: vk::AccessFlags::SHADER_WRITE, 
                    destination_access: vk::AccessFlags::TRANSFER_READ, 
                    source_stage: vk::PipelineStageFlags::COMPUTE_SHADER, 
                    destination_stage: vk::PipelineStageFlags::TRANSFER 
                })
                .blit_image(&self.output_texture.image, &self.ctx.swapchain.images[image_index as usize], vk::ImageLayout::TRANSFER_SRC_OPTIMAL, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageAspectFlags::COLOR, vk::Filter::NEAREST)
                .transition_image_layout(&self.ctx.swapchain.images[image_index as usize], &TransitionLayoutOptions { 
                    old: vk::ImageLayout::TRANSFER_DST_OPTIMAL, 
                    new: vk::ImageLayout::PRESENT_SRC_KHR, 
                    source_access: vk::AccessFlags::TRANSFER_WRITE, 
                    destination_access: vk::AccessFlags::NONE, 
                    source_stage: vk::PipelineStageFlags::TRANSFER, 
                    destination_stage: vk::PipelineStageFlags::BOTTOM_OF_PIPE 
                })
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
