use crate::renderer::Renderer;
use ash::vk;
use vulkan::{Set, Texture};

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
