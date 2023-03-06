use super::{Device, GraphicsPipeline, Renderpass};
use ash::{prelude::*, vk};
use std::{ops::Deref, result::Result};

#[derive(Clone, Copy, Debug, Default)]
pub struct DrawOptions {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

pub struct CommandBuffer {
    pub(crate) buffer: vk::CommandBuffer,
}

impl CommandBuffer {
    pub fn begin(self, device: &Device) -> Result<Self, vk::Result> {
        let begin_info = vk::CommandBufferBeginInfo::builder();
        unsafe { device.begin_command_buffer(*self, &begin_info)? };
        Ok(self)
    }

    pub fn begin_renderpass(
        self,
        device: &Device,
        renderpass: &Renderpass,
        framebuffer: &vk::Framebuffer,
        extent: vk::Extent2D,
    ) -> Self {
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(extent);

        let color_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let clear_values = &[color_clear_value];
        let begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(**renderpass)
            .framebuffer(*framebuffer)
            .render_area(*render_area)
            .clear_values(clear_values);

        unsafe { device.cmd_begin_render_pass(*self, &begin_info, vk::SubpassContents::INLINE) };

        self
    }

    pub fn bind_pipeline(self, device: &Device, pipeline: &GraphicsPipeline) -> Self {
        unsafe { device.cmd_bind_pipeline(*self, vk::PipelineBindPoint::GRAPHICS, **pipeline) };

        self
    }

    pub fn draw(self, device: &Device, options: DrawOptions) -> Self {
        unsafe {
            device.cmd_draw(
                *self,
                options.vertex_count,
                options.instance_count,
                options.first_vertex,
                options.first_instance,
            )
        };

        self
    }

    pub fn end_renderpass(self, device: &Device) -> Self {
        unsafe { device.cmd_end_render_pass(*self) };

        self
    }

    pub fn end(self, device: &Device) -> Result<Self, vk::Result> {
        unsafe { device.end_command_buffer(*self)? };

        Ok(self)
    }
}

impl Deref for CommandBuffer {
    type Target = vk::CommandBuffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

pub struct CommandPool {
    pub(crate) pool: vk::CommandPool,
}

impl CommandPool {
    pub fn new(device: &Device) -> Result<Self, vk::Result> {
        let create_info =
            vk::CommandPoolCreateInfo::builder().queue_family_index(device.queues.graphics.index);

        let pool = unsafe { device.create_command_pool(&create_info, None)? };

        Ok(Self { pool })
    }

    pub fn allocate(&self, device: &Device) -> Result<CommandBuffer, vk::Result> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let buffer = unsafe { device.allocate_command_buffers(&allocate_info)?[0] };

        Ok(CommandBuffer { buffer })
    }
}

impl Deref for CommandPool {
    type Target = vk::CommandPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
