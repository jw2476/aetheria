use crate::renderer::Renderer;
use ash::vk;
use bevy_ecs::prelude::*;
use bytemuck::{bytes_of, cast_slice, Pod, Zeroable};
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use std::{collections::HashMap, hash::Hash, path::Path};
use vulkan::{Buffer, Context, Set, Texture};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Vec3,
    pub uv: Vec2,
    pub normal: Vec3,
}

pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub material: Option<MaterialRef>,
}

impl Mesh {
    pub fn new(
        ctx: &Context,
        vertices: &[Vertex],
        indices: &[u32],
        material: Option<MaterialRef>,
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
            material,
        })
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

        let model = cast_slice::<f32, u8>(&model.to_cols_array()).to_vec();
        self.buffer.upload(model);

        Ok(())
    }
}

#[derive(Clone, Copy)]
struct MaterialData {
    base_color_factor: Vec4,
}

impl MaterialData {
    pub fn to_bytes(&self) -> Vec<u8> {
        cast_slice::<f32, u8>(&self.base_color_factor.to_array()).to_vec()
    }
}

pub struct Material {
    pub base_color_texture: TextureRef,

    data: MaterialData,
    pub buffer: Buffer,
    pub set: Set,
}

impl Material {
    pub fn new(
        world: &mut World,
        base_color_factor: Vec4,
        base_color_texture: TextureRef,
    ) -> Result<Self, vk::Result> {
        let data = MaterialData { base_color_factor };

        let bytes = data.to_bytes();
        let buffer = Buffer::new(
            &world.get_resource::<Renderer>().unwrap().ctx,
            bytes,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;
        let set = world
            .get_resource_mut::<Renderer>()
            .unwrap()
            .material_pool
            .allocate()?;
        set.update_buffer(
            &world.get_resource::<Renderer>().unwrap().ctx.device,
            0,
            &buffer,
        );
        set.update_texture(
            &world.get_resource::<Renderer>().unwrap().ctx.device,
            1,
            &world
                .get_resource::<TextureRegistry>()
                .unwrap()
                .get(&base_color_texture)
                .unwrap(),
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        Ok(Self {
            base_color_texture,
            data,
            buffer,
            set,
        })
    }
}

pub struct EguiTexture {
    texture: Texture,
    pub set: Set,
}

impl EguiTexture {
    pub fn new(
        renderer: &mut Renderer,
        bytes: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Self, vk::Result> {
        let mut texture = Texture::new_bytes(&mut renderer.ctx, bytes, width, height)?;
        texture.sampler = texture.image.create_sampler(
            &renderer.ctx,
            vk::Filter::NEAREST,
            vk::Filter::NEAREST,
        )?;
        let set = renderer.egui_texture_pool.allocate()?;
        set.update_texture(
            &renderer.ctx.device,
            0,
            &texture,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        Ok(Self { texture, set })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Component, Hash, Eq, PartialEq)]
pub struct MeshRef(usize);

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct TextureRef(usize);

impl TextureRef {
    pub const WHITE: TextureRef = Self(0);
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Component, Hash, Eq, PartialEq)]
pub struct TransformRef(usize);

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct MaterialRef(usize);

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct EguiTextureRef(usize);

impl From<usize> for MeshRef {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<usize> for TransformRef {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<usize> for TextureRef {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<usize> for MaterialRef {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<usize> for EguiTextureRef {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<egui::TextureId> for EguiTextureRef {
    fn from(value: egui::TextureId) -> Self {
        match value {
            egui::TextureId::Managed(id) => Self(id.try_into().unwrap()),
            egui::TextureId::User(id) => Self(id.try_into().unwrap()),
        }
    }
}

#[derive(Resource)]
pub struct Registry<R, T> {
    pub registry: HashMap<R, T>,
    next_id: usize,
}

impl<R: Hash + Eq + From<usize> + Clone, T> Registry<R, T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, data: T) -> R {
        let r: R = self.next_id.into();
        self.registry.insert(r.clone(), data);
        self.next_id += 1;
        r
    }

    pub fn get(&self, data_ref: &R) -> Option<&T> {
        self.registry.get(data_ref)
    }

    pub fn get_mut(&mut self, data_ref: &R) -> Option<&mut T> {
        self.registry.get_mut(data_ref)
    }
}

impl<R, T> Default for Registry<R, T> {
    fn default() -> Self {
        Self {
            registry: HashMap::new(),
            next_id: 0,
        }
    }
}

pub type MeshRegistry = Registry<MeshRef, Mesh>;
pub type TextureRegistry = Registry<TextureRef, Texture>;
pub type TransformRegistry = Registry<TransformRef, Transform>;
pub type MaterialRegistry = Registry<MaterialRef, Material>;
pub type EguiTextureRegistry = Registry<EguiTextureRef, EguiTexture>;
