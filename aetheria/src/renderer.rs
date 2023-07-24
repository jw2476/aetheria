use ash::vk;
use assets::{Mesh, MeshRegistry, ShaderRegistry, Vertex};
use bytemuck::{cast_slice, cast_slice_mut, Pod, Zeroable};
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::{ops::Deref, sync::Arc};
use tracing::info;
use vulkan::command::{self, TransitionLayoutOptions};
use vulkan::VertexInputBuilder;
use vulkan::{
    compute, graphics, Buffer, Context, Image, Pool, Set, SetLayout, SetLayoutBuilder, Shader,
    Swapchain, Texture,
};
use winit::window::Window;

use crate::camera::Camera;
use crate::time::Time;
use crate::transform::Transform;

pub struct RenderObjectBuilder<'a> {
    renderer: &'a mut Renderer,
    mesh_registry: &'a mut MeshRegistry,
    mesh: Option<Arc<Mesh>>,
    color: Option<Vec3>,
    transform: Option<Transform>,
}

impl RenderObjectBuilder<'_> {
    pub fn set_mesh(&mut self, path: &str) -> Result<&mut Self, vk::Result> {
        self.mesh = Some(self.mesh_registry.load(self.renderer, path));
        Ok(self)
    }

    pub fn set_color(&mut self, color: Vec3) -> &mut Self {
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

        let material = Material {
            albedo: self.color.unwrap_or_else(|| Vec3::new(1.0, 1.0, 1.0)),
            roughness: 1.0,
            metalness: 0.0,
            ..Default::default()
        };
        let transform = self.transform.clone().unwrap_or(Transform::IDENTITY);

        Ok(RenderObject {
            mesh: self.mesh.clone().unwrap(),
            material,
            transform,
        })
    }
}

#[derive(Clone)]
pub struct RenderObject {
    mesh: Arc<Mesh>,
    material: Material,
    pub transform: Transform,
}

impl RenderObject {
    pub fn builder<'a>(
        renderer: &'a mut Renderer,
        mesh_registry: &'a mut MeshRegistry,
    ) -> RenderObjectBuilder<'a> {
        RenderObjectBuilder {
            renderer,
            mesh_registry,
            mesh: None,
            color: None,
            transform: None,
        }
    }
}

pub trait Renderable {
    fn get_objects(&self) -> Vec<&RenderObject>;
}

impl<T: Renderable> Renderable for Vec<T> {
    fn get_objects(&self) -> Vec<&RenderObject> {
        self.iter()
            .flat_map(|item| item.get_objects())
            .collect::<Vec<&RenderObject>>()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct MeshData {
    first_index: i32,
    num_indices: i32,
    material: i32,
    _padding: [f32; 1],
    min_aabb: [f32; 3],
    _padding2: [f32; 1],
    max_aabb: [f32; 3],
    _padding3: [f32; 1],
    transform: [f32; 16],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct Material {
    albedo: Vec3,
    roughness: f32,
    metalness: f32,
    _padding: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Light {
    pub position: Vec3,
    pub strength: f32,
    pub color: Vec3,
    _padding: [f32; 1],
}

impl Light {
    pub fn new(position: Vec3, strength: f32, color: Vec3) -> Self {
        Self {
            position,
            strength,
            color,
            _padding: [0.0],
        }
    }
}

pub struct RenderPass {
    texture: Texture,

    frame_layout: SetLayout,
    frame_pool: Pool,
    frame_set: Set,

    geometry_layout: SetLayout,
    geometry_pool: Pool,
    geometry_set: Set,
    pipeline: compute::Pipeline,
}

impl RenderPass {
    pub fn new(
        ctx: &Context,
        shader_registry: &mut ShaderRegistry,
        camera: &Camera,
        time: &Time,
    ) -> Result<Self, vk::Result> {
        let image = Image::new(
            &ctx,
            RENDER_WIDTH,
            RENDER_HEIGHT,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
        )?;
        let texture = Texture::from_image(&ctx, image, vk::Filter::NEAREST, vk::Filter::NEAREST)?;

        let frame_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::UNIFORM_BUFFER)
            .add(vk::DescriptorType::UNIFORM_BUFFER)
            .build()?;
        let mut frame_pool = Pool::new(ctx.device.clone(), frame_layout.clone(), 1)?;
        let frame_set = frame_pool.allocate()?;
        frame_set.update_buffer(&ctx.device, 0, &camera.buffer);
        frame_set.update_buffer(&ctx.device, 1, &time.buffer);

        let geometry_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::STORAGE_IMAGE)
            .add(vk::DescriptorType::STORAGE_BUFFER)
            .add(vk::DescriptorType::STORAGE_BUFFER)
            .add(vk::DescriptorType::STORAGE_BUFFER)
            .add(vk::DescriptorType::STORAGE_BUFFER)
            .add(vk::DescriptorType::STORAGE_BUFFER)
            .build()?;
        let mut geometry_pool = Pool::new(ctx.device.clone(), geometry_layout.clone(), 1)?;
        let geometry_set = geometry_pool.allocate()?;
        geometry_set.update_texture(&ctx.device, 0, &texture, vk::ImageLayout::GENERAL);

        let shader: Arc<Shader> = shader_registry.load(&ctx.device, "test.comp.glsl");
        let pipeline = compute::Pipeline::new(
            &ctx.device,
            shader.clone(),
            &[frame_layout.clone(), geometry_layout.clone()],
        )?;

        Ok(Self {
            texture,
            frame_layout,
            frame_set,
            frame_pool,
            geometry_layout,
            geometry_pool,
            geometry_set,
            pipeline,
        })
    }

    fn calculate_box(object: &RenderObject) -> (Vec3, Vec3) {
        let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for vertex in &object.mesh.vertices {
            let v = object.transform.get_matrix()
                * Vec4::new(vertex.pos.x, vertex.pos.y, vertex.pos.z, 1.0);
            min.x = min.x.min(v.x);
            min.y = min.y.min(v.y);
            min.z = min.z.min(v.z);

            max.x = max.x.max(v.x);
            max.y = max.y.max(v.y);
            max.z = max.z.max(v.z);
        }

        return (min, max);
    }

    pub fn set_geometry(
        &self,
        renderer: &Renderer,
        mesh_registry: &MeshRegistry,
        renderables: &[&dyn Renderable],
        lights: &[Light],
    ) {
        let objects = renderables
            .iter()
            .flat_map(|renderable| renderable.get_objects())
            .collect::<Vec<&RenderObject>>();

        let mut meshes: Vec<MeshData> = Vec::new();
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<i32> = Vec::new();
        let mut materials: Vec<Material> = Vec::new();

        let mut mesh_to_index: HashMap<*const Mesh, i32> = HashMap::new();

        for mesh in &mesh_registry.get_meshes() {
            mesh_to_index.insert(Arc::as_ptr(&mesh), indices.len() as i32);
            indices.append(
                &mut mesh
                    .indices
                    .iter()
                    .copied()
                    .map(|index| index as i32 + vertices.len() as i32)
                    .collect::<Vec<i32>>(),
            );
            vertices.append(&mut mesh.vertices.clone());
        }

        for (i, object) in objects.iter().enumerate() {
            let (min_aabb, max_aabb) = Self::calculate_box(&object);

            let transform = object.transform.get_matrix().to_cols_array();
            let mesh = MeshData {
                first_index: *mesh_to_index
                    .get(&Arc::as_ptr(&object.mesh))
                    .expect("Can't find index in mesh_to_index"),
                num_indices: object.mesh.indices.len() as i32,
                material: i as i32,
                transform,
                min_aabb: min_aabb.to_array(),
                max_aabb: max_aabb.to_array(),
                ..Default::default()
            };
            meshes.push(mesh);
            materials.push(object.material);
        }

        let mut mesh_data = cast_slice::<i32, u8>(&[meshes.len() as i32, 0, 0, 0]).to_vec();
        mesh_data.append(&mut cast_slice::<MeshData, u8>(&meshes).to_vec());

        let vertex_buffer = Buffer::new(
            &renderer,
            cast_slice::<Vertex, u8>(&vertices),
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )
        .unwrap();
        let indices = indices
            .iter()
            .copied()
            .flat_map(|index| [index, 0, 0, 0])
            .collect::<Vec<i32>>();
        let index_buffer = Buffer::new(
            &renderer,
            cast_slice::<i32, u8>(&indices),
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )
        .unwrap();
        let mesh_buffer =
            Buffer::new(&renderer, mesh_data, vk::BufferUsageFlags::STORAGE_BUFFER).unwrap();
        let material_buffer = Buffer::new(
            &renderer,
            cast_slice::<Material, u8>(&materials),
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )
        .unwrap();

        let mut light_data = cast_slice::<Light, u8>(lights).to_vec();
        let mut light_buffer = cast_slice::<i32, u8>(&[lights.len() as i32, 0, 0, 0]).to_vec();
        light_buffer.append(&mut light_data);
        let light_buffer = Buffer::new(
            &renderer,
            light_buffer,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )
        .unwrap();

        self.geometry_set
            .update_buffer(&renderer.device, 1, &vertex_buffer);
        self.geometry_set
            .update_buffer(&renderer.device, 2, &index_buffer);
        self.geometry_set
            .update_buffer(&renderer.device, 3, &mesh_buffer);
        self.geometry_set
            .update_buffer(&renderer.device, 4, &material_buffer);
        self.geometry_set
            .update_buffer(&renderer.device, 5, &light_buffer);
    }

    pub fn get_texture(&self) -> &'_ Texture {
        &self.texture
    }
}

impl Pass for RenderPass {
    fn record(&self, cmd: command::BufferBuilder) -> command::BufferBuilder {
        cmd.transition_image_layout(
            &self.texture.image,
            &TransitionLayoutOptions {
                old: vk::ImageLayout::UNDEFINED,
                new: vk::ImageLayout::GENERAL,
                source_access: vk::AccessFlags::NONE,
                destination_access: vk::AccessFlags::SHADER_WRITE,
                source_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                destination_stage: vk::PipelineStageFlags::COMPUTE_SHADER,
            },
        )
        .bind_compute_pipeline(self.pipeline.clone())
        .bind_descriptor_set(0, &self.frame_set)
        .bind_descriptor_set(1, &self.geometry_set)
        .dispatch(
            RENDER_WIDTH / 16,
            (RENDER_HEIGHT as f32 / 16.0).ceil() as u32,
            1,
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Rectangle {
    pub color: Vec4,
    pub origin: Vec2,
    pub extent: Vec2,
    pub radius: f32,
    pub _padding: [u8; 12],
}

pub struct UIPass {
    pipeline: compute::Pipeline,
    ui_layout: SetLayout,
    ui_pool: Pool,
    ui_set: Set,
    output: Texture,
}

impl UIPass {
    pub fn new(
        ctx: &Context,
        shader_registry: &mut ShaderRegistry,
        input: &Texture,
    ) -> Result<Self, vk::Result> {
        let image = Image::new(
            &ctx,
            RENDER_WIDTH,
            RENDER_HEIGHT,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC,
        )?;
        let output = Texture::from_image(&ctx, image, vk::Filter::NEAREST, vk::Filter::NEAREST)?;

        let ui_layout = SetLayoutBuilder::new(&ctx.device)
            .add(vk::DescriptorType::STORAGE_IMAGE)
            .add(vk::DescriptorType::STORAGE_IMAGE)
            .add(vk::DescriptorType::STORAGE_BUFFER)
            .build()?;
        let mut ui_pool = Pool::new(ctx.device.clone(), ui_layout.clone(), 1)?;
        let ui_set = ui_pool.allocate()?;
        ui_set.update_texture(&ctx.device, 0, &output, vk::ImageLayout::GENERAL);
        ui_set.update_texture(&ctx.device, 1, &input, vk::ImageLayout::GENERAL);

        let shader: Arc<Shader> = shader_registry.load(&ctx.device, "ui.comp.glsl");
        let pipeline = compute::Pipeline::new(&ctx.device, shader.clone(), &[ui_layout.clone()])?;

        Ok(Self {
            pipeline,
            ui_layout,
            ui_pool,
            ui_set,
            output,
        })
    }

    pub fn set_geometry(&self, ctx: &Context, rectangles: &[Rectangle]) -> Result<(), vk::Result> {
        let mut rectangle_data: Vec<u8> =
            cast_slice::<i32, u8>(&[rectangles.len() as i32, 0, 0, 0]).to_vec();
        println!("{:?}", rectangle_data);
        rectangle_data.extend_from_slice(cast_slice::<Rectangle, u8>(rectangles));
        let rectangle_buffer =
            Buffer::new(ctx, rectangle_data, vk::BufferUsageFlags::STORAGE_BUFFER)?;
        self.ui_set.update_buffer(&ctx.device, 2, &rectangle_buffer);
        Ok(())
    }

    pub fn get_texture(&self) -> &'_ Texture {
        &self.output
    }
}

impl Pass for UIPass {
    fn record(&self, cmd: command::BufferBuilder) -> command::BufferBuilder {
        cmd.transition_image_layout(
            &self.output.image,
            &TransitionLayoutOptions {
                old: vk::ImageLayout::UNDEFINED,
                new: vk::ImageLayout::GENERAL,
                source_access: vk::AccessFlags::NONE,
                destination_access: vk::AccessFlags::SHADER_WRITE,
                source_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                destination_stage: vk::PipelineStageFlags::COMPUTE_SHADER,
            },
        )
        .bind_compute_pipeline(self.pipeline.clone())
        .bind_descriptor_set(0, &self.ui_set)
        .dispatch(
            RENDER_WIDTH / 16,
            (RENDER_HEIGHT as f32 / 16.0).ceil() as u32,
            1,
        )
    }
}

pub trait Pass {
    fn record(&self, cmd: command::BufferBuilder) -> command::BufferBuilder;
}

pub struct Renderer {
    pub(crate) ctx: Context,
    window: Arc<Window>,

    render_finished: vk::Semaphore,
    in_flight: vk::Fence,
    output_image: Option<(Arc<Image>, vk::ImageLayout)>,

    passes: Vec<Arc<dyn Pass>>,
}

const RENDER_WIDTH: u32 = 480;
const RENDER_HEIGHT: u32 = 270;

impl Renderer {
    pub fn new(ctx: Context, window: Arc<Window>) -> Result<Self, vk::Result> {
        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let render_finished =
            unsafe { ctx.device.create_semaphore(&semaphore_info, None).unwrap() };
        let in_flight = unsafe { ctx.device.create_fence(&fence_info, None).unwrap() };

        let renderer = Self {
            ctx,
            window,
            render_finished,
            in_flight,
            output_image: None,
            passes: Vec::new(),
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

    pub fn add_pass(&mut self, pass: Arc<dyn Pass>) {
        self.passes.push(pass);
    }

    pub fn set_output_image(&mut self, image: Arc<Image>, layout: vk::ImageLayout) {
        self.output_image = Some((image, layout));
    }

    pub fn wait_for_frame(&self) {
        unsafe {
            self.device
                .wait_for_fences(&[self.in_flight], true, u64::MAX)
                .unwrap();
        }
    }

    pub fn render(&mut self) {
        unsafe {
            let in_flight = self.in_flight.clone();

            let acquire_result = self.ctx.start_frame(in_flight);

            /*self.render_pass
                .set_geometry(&self, mesh_registry, renderables, lights);
            self.ui_pass.set_geometry(&self, &[Rectangle { origin: Vec2::new(50.0, 50.0), extent: Vec2::new(50.0, 50.0), radius: 25.0, color: Vec4::new(1.0, 0.0, 1.0, 0.3), ..Default::default() }]).expect("Failed to update UI geometry");*/

            let image_index = match acquire_result {
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.recreate_swapchain()
                        .expect("Swapchain recreation failed");
                    return;
                }
                Err(e) => panic!("{}", e),
                Ok(image_index) => image_index,
            };

            self.command_pool.clear();

            let cmd = self
                .command_pool
                .allocate()
                .unwrap()
                .begin()
                .unwrap()
                .record(|cmd| {
                    self.passes
                        .iter()
                        .fold(cmd, |cmd, pass| cmd.record(|cmd| pass.record(cmd)))
                })
                .transition_image_layout(
                    &self.output_image.as_ref().expect("No output image set").0,
                    &TransitionLayoutOptions {
                        old: self.output_image.as_ref().unwrap().1,
                        new: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                        source_access: vk::AccessFlags::SHADER_WRITE,
                        destination_access: vk::AccessFlags::TRANSFER_READ,
                        source_stage: vk::PipelineStageFlags::COMPUTE_SHADER,
                        destination_stage: vk::PipelineStageFlags::TRANSFER,
                    },
                )
                .transition_image_layout(
                    &self.ctx.swapchain.images[image_index as usize],
                    &TransitionLayoutOptions {
                        old: vk::ImageLayout::UNDEFINED,
                        new: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        source_access: vk::AccessFlags::NONE,
                        destination_access: vk::AccessFlags::TRANSFER_WRITE,
                        source_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                        destination_stage: vk::PipelineStageFlags::TRANSFER,
                    },
                )
                .blit_image(
                    &self.output_image.as_ref().unwrap().0,
                    &self.ctx.swapchain.images[image_index as usize],
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageAspectFlags::COLOR,
                    vk::Filter::NEAREST,
                )
                .transition_image_layout(
                    &self.ctx.swapchain.images[image_index as usize],
                    &TransitionLayoutOptions {
                        old: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        new: vk::ImageLayout::PRESENT_SRC_KHR,
                        source_access: vk::AccessFlags::TRANSFER_WRITE,
                        destination_access: vk::AccessFlags::NONE,
                        source_stage: vk::PipelineStageFlags::TRANSFER,
                        destination_stage: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    },
                )
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

            self.ctx
                .device
                .queue_submit(
                    self.ctx.device.queues.graphics.queue,
                    &[*submit_info],
                    self.in_flight,
                )
                .unwrap();

            let presentation_result = self.ctx.end_frame(image_index, self.render_finished);

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
