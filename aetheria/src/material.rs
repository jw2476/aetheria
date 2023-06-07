use ash::vk;
use bytemuck::cast_slice;
use glam::Vec4;
use vulkan::{Set, Buffer};

use crate::renderer::Renderer;

pub struct Material {
    buffer: Buffer,
    pub set: Set,
}

impl Material {
    pub fn new(renderer: &mut Renderer, base_color: Vec4) -> Result<Material, vk::Result> {
         let buffer = Buffer::new(&renderer.ctx, cast_slice::<f32, u8>(&base_color.to_array()), vk::BufferUsageFlags::UNIFORM_BUFFER)?;
         let set = renderer.material_pool.allocate()?;
         set.update_buffer(&renderer.ctx.device, 0, &buffer);

         Ok(Self { 
             buffer, 
             set, 
         })
    }
}
