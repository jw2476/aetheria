use ash::vk;
use bevy_ecs::system::Resource;
use bytemuck::cast_slice;
use glam::{Mat4, Vec3};
use vulkan::{Buffer, Set};

use crate::renderer::Renderer;

#[derive(Resource)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,

    buffer: Buffer,
    pub set: Set,
}

const DEFAULT_ZOOM: f32 = 1000.0;

impl Camera {
    pub fn new(renderer: &mut Renderer) -> Result<Self, vk::Result> {
        let eye = Vec3::new(0.0, 5.0 * 35.264_f32.tan(), 5.0);
        let target = Vec3::new(0.0, 0.5, 0.0);

        let default = [0_u8; 128];
        let set = renderer.camera_pool.allocate()?;
        let buffer = Buffer::new(
            &renderer.ctx,
            default.to_vec(),
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;
        set.update_buffer(&renderer.ctx.device, 0, &buffer);

        let mut camera = Self {
            eye,
            target,
            buffer,
            set,
        };

        camera.update(
            renderer.swapchain.extent.width as f32,
            renderer.swapchain.extent.height as f32,
        );

        Ok(camera)
    }

    pub fn update(&mut self, width: f32, height: f32) {
        let view = Mat4::look_at_rh(self.eye, self.target, Vec3::new(0.0, 1.0, 0.0));
        let mut proj = Mat4::orthographic_rh(-width/DEFAULT_ZOOM, width/DEFAULT_ZOOM, -height/DEFAULT_ZOOM, height/DEFAULT_ZOOM, 0.1, 100.0);

        proj.col_mut(1)[1] *= -1.0;

        let vp = [view.to_cols_array(), proj.to_cols_array()]
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<f32>>();
        let vp = cast_slice::<f32, u8>(&vp);
        self.buffer.upload(vp.to_vec());
    }
}
