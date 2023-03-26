use std::path::Path;
use ash::vk;
use vulkan::{Buffer, Context, Set};
use bevy_ecs::prelude::*;
use bytemuck::{bytes_of, cast_slice, Pod, Zeroable};
use glam::{Mat4, Quat, Vec2, Vec3};
use crate::renderer::Renderer;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Vec3,
    pub uv:  Vec2
}

pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub texture: Option<TextureRef>
}

impl Mesh {
    pub fn new(ctx: &Context, vertices: &[Vertex], indices: &[u32], texture: Option<TextureRef>) -> Result<Self, vk::Result> {
        let vertex_buffer = Buffer::new(ctx, cast_slice(vertices), vk::BufferUsageFlags::VERTEX_BUFFER)?;
        let index_buffer = Buffer::new(ctx, cast_slice(indices), vk::BufferUsageFlags::INDEX_BUFFER)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            texture
        })
    }
}

pub struct Texture {
    texture: vulkan::Texture,
    pub set: Set
}

impl Texture {
    pub fn new(renderer: &mut Renderer, path: &Path) -> Result<Self, vk::Result> {
        let texture = vulkan::Texture::new(&mut renderer.ctx, path)?;
        let set = renderer.transform_pool.allocate()?;
        set.update_texture(&renderer.device, 0, &texture, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        Ok(Self {
            texture,
            set
        })
    }
}

pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,

    buffer: Buffer,
    pub set: Set
}

impl Transform {
    pub fn new(renderer: &mut Renderer) -> Result<Self, vk::Result> {
        let translation = Vec3::new(0.0, 0.0, 0.0);
        let rotation = Quat::IDENTITY;
        let scale = Vec3::new(1.0, 1.0, 1.0);

        let matrix = Mat4::from_scale_rotation_translation(scale, rotation, translation);

        let buffer = Buffer::new(renderer, cast_slice(&matrix.to_cols_array()), vk::BufferUsageFlags::UNIFORM_BUFFER)?;
        let set = renderer.transform_pool.allocate()?;
        set.update_buffer(&renderer.device, 0, &buffer);

        Ok(Self {
            translation,
            rotation,
            scale,

            buffer,
            set
        })
    }

    pub fn get_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Component)]
pub struct MeshRef(usize);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TextureRef(usize);

#[repr(C)]
#[derive(Clone, Copy, Debug, Component)]
pub struct TransformRef(usize);

impl From<usize> for MeshRef {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<MeshRef> for usize {
    fn from(value: MeshRef) -> Self {
        value.0
    }
}

impl From<usize> for TextureRef {
    fn from(value: usize) -> Self {
        Self(value)
    }
}
impl From<TextureRef> for usize {
    fn from(value: TextureRef) -> Self {
        value.0
    }
}

impl From<usize> for TransformRef {
    fn from(value: usize) -> Self {
        Self(value)
    }
}
impl From<TransformRef> for usize {
    fn from(value: TransformRef) -> Self {
        value.0
    }
}

#[derive(Resource)]
pub struct Registry<T> {
    pub registry: Vec<T>
}

impl<T> Registry<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<R: From<usize>>(&mut self, data: T) -> R {
        self.registry.push(data);
        self.registry.len().into()
    }

    pub fn get<R: Into<usize>>(&self, data_ref: R) -> Option<&T> {
        self.registry.get(data_ref.into())
    }
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self {
            registry: Vec::new()
        }
    }
}

pub type MeshRegistry = Registry<Mesh>;
pub type TextureRegistry = Registry<Texture>;
pub type TransformRegistry = Registry<Transform>;
