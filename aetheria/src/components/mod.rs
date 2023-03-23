use std::rc::Rc;
use std::sync::{Arc, Mutex};
use ash::vk;
use glam::{Mat4, Quat, Vec2, Vec3};
use bevy_ecs::component::Component;
use bytemuck::{bytes_of, cast_slice, NoUninit};
use vulkan::{Context, Set, Texture, Buffer};
use vulkan::command::BufferBuilder;
use crate::renderer::Renderer;

#[repr(C)]
#[derive(Copy, Clone, Debug, NoUninit)]
pub struct Vertex {
    pub(crate) pos: Vec3,
    pub(crate) uv: Vec2
}

#[derive(Component)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub texture: Option<Texture>
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>, texture: Option<Texture>) -> Result<Self, vk::Result> {
        Ok(Self {
            vertices,
            indices,
            texture
        })
    }
}



#[repr(C)]
#[derive(Component, Copy, Clone, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::default(),
            scale: Vec3::new(1.0, 1.0, 1.0)
        }
    }
}