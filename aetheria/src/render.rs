use ash::vk;
use assets::{Mesh, MeshRegistry, ShaderRegistry, Vertex};
use bytemuck::{cast_slice, Pod, Zeroable};
use glam::{Vec3, Vec4};
use std::{collections::HashMap, sync::Arc};
use vulkan::{
    command, command::TransitionLayoutOptions, compute, Buffer, Context, Image, Pool, Set,
    SetLayout, SetLayoutBuilder, Shader, Texture,
};

use crate::{
    renderer::{Pass, Renderer, RENDER_HEIGHT, RENDER_WIDTH},
    transform::Transform,
    Camera, Time,
};

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
