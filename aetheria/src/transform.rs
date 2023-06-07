use bytemuck::cast_slice;
use glam::{Vec3, Quat, Mat4};
use vulkan::{Buffer, Set};
use ash::vk;

use crate::renderer::Renderer;

#[derive(Clone)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {  
    pub const IDENTITY: Self = Self { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE };

    pub fn get_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

pub struct TransformGPU {
    pub transform: Transform,
    buffer: Buffer,
    pub set: Set
}

impl TransformGPU {
    pub fn new(renderer: &mut Renderer, transform: Transform) -> Result<Self, vk::Result> {
        let placeholder = vec![0_u8; 192];
        let buffer = Buffer::new(renderer, placeholder, vk::BufferUsageFlags::UNIFORM_BUFFER)?;
        let set = renderer.transform_pool.allocate()?;
        set.update_buffer(&renderer.device, 0, &buffer);

        let mut gpu = Self {
            transform,
            buffer,
            set,
        };

        gpu.update()?;

        Ok(gpu)
    }

    pub fn update(&mut self) -> Result<(), vk::Result> {
        let model = self.transform.get_matrix();

        let model = cast_slice::<f32, u8>(&model.to_cols_array()).to_vec();
        self.buffer.upload(model);

        Ok(())
    }
}
