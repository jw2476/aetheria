use crate::renderer::Renderer;
use ash::vk;
use bevy_ecs::prelude::*;
use bytemuck::{bytes_of, cast_slice, Pod, Zeroable};
use glam::{Mat4, Quat, Vec2, Vec3};
use std::path::Path;
use vulkan::{Buffer, Context, Set};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Vec3,
    pub uv: Vec2,
}

pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub texture: Option<TextureRef>,
}

impl Mesh {
    pub fn new(
        ctx: &Context,
        vertices: &[Vertex],
        indices: &[u32],
        texture: Option<TextureRef>,
    ) -> Result<Self, vk::Result> {
        let vertex_buffer = Buffer::new(
            ctx,
            cast_slice(vertices),
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let index_buffer =
            Buffer::new(ctx, cast_slice(indices), vk::BufferUsageFlags::INDEX_BUFFER)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            texture,
        })
    }
}

pub struct Texture {
    texture: vulkan::Texture,
    pub set: Set,
}

impl Texture {
    pub fn new(renderer: &mut Renderer, path: &Path) -> Result<Self, vk::Result> {
        let texture = vulkan::Texture::new(&mut renderer.ctx, path)?;
        let set = renderer.texture_pool.allocate()?;
        set.update_texture(
            &renderer.device,
            0,
            &texture,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        Ok(Self { texture, set })
    }
}

pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,

    buffer: Buffer,
    pub set: Set,
}

impl Transform {
    pub fn new(renderer: &mut Renderer) -> Result<Self, vk::Result> {
        let translation = Vec3::new(0.0, 0.0, 0.0);
        let rotation = Quat::IDENTITY;
        let scale = Vec3::new(1.0, 1.0, 1.0);

        let placeholder = vec![0_u8; 192];
        let buffer = Buffer::new(renderer, placeholder, vk::BufferUsageFlags::UNIFORM_BUFFER)?;
        let set = renderer.transform_pool.allocate()?;
        set.update_buffer(&renderer.device, 0, &buffer);

        let mut transform = Self {
            translation,
            rotation,
            scale,

            buffer,
            set,
        };

        transform.update(renderer);

        Ok(transform)
    }

    pub fn update(&mut self, renderer: &Renderer) -> Result<(), vk::Result> {
        let model =
            Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation);
        let view = Mat4::look_at_rh(
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );
        let mut projection = Mat4::perspective_rh(
            45.0_f32.to_radians(),
            renderer.swapchain.extent.width as f32 / renderer.swapchain.extent.height as f32,
            0.01,
            1000.0,
        );

        projection.col_mut(1)[1] *= -1.0;

        let mvp: [Mat4; 3] = [model, view, projection];
        let mvp: Vec<f32> = mvp
            .iter()
            .flat_map(|matrix| matrix.to_cols_array())
            .collect();
        let mvp: &[u8] = cast_slice(&mvp);

        self.buffer.upload(mvp);

        Ok(())
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
    pub registry: Vec<T>,
}

impl<T> Registry<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<R: From<usize>>(&mut self, data: T) -> R {
        self.registry.push(data);
        (self.registry.len() - 1).into()
    }

    pub fn get<R: Into<usize>>(&self, data_ref: R) -> Option<&T> {
        self.registry.get(data_ref.into())
    }

    pub fn get_mut<R: Into<usize>>(&mut self, data_ref: R) -> Option<&mut T> {
        self.registry.get_mut(data_ref.into())
    }
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self {
            registry: Vec::new(),
        }
    }
}

pub type MeshRegistry = Registry<Mesh>;
pub type TextureRegistry = Registry<Texture>;
pub type TransformRegistry = Registry<Transform>;
