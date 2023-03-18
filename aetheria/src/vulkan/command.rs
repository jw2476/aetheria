use super::{Set, Device, Pipeline, Renderpass};
use ash::vk;
use std::{
    ops::Deref,
    result::Result,
};
use crate::vulkan::Image;

#[derive(Clone, Copy, Debug, Default)]
pub struct DrawOptions {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: i32,
    pub first_instance: u32,
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub(crate) buffer: vk::CommandBuffer,
}

#[derive(Clone)]
pub struct BufferBuilder<'a> {
    buffer: Buffer,
    device: &'a Device
}

#[derive(Clone, Debug)]
pub struct TransitionLayoutOptions {
    pub old: vk::ImageLayout,
    pub new: vk::ImageLayout,
    pub source_access: vk::AccessFlags,
    pub destination_access: vk::AccessFlags,
    pub source_stage: vk::PipelineStageFlags,
    pub destination_stage: vk::PipelineStageFlags
}


impl BufferBuilder<'_> {
    pub fn begin(self) -> Result<Self, vk::Result> {
        let begin_info = vk::CommandBufferBeginInfo::builder();
        unsafe { self.device.begin_command_buffer(**self, &begin_info)? };
        Ok(self)
    }

    pub fn begin_renderpass(
        self,
        renderpass: &Renderpass,
        framebuffer: vk::Framebuffer,
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
            .framebuffer(framebuffer)
            .render_area(*render_area)
            .clear_values(clear_values);

        unsafe { self.device.cmd_begin_render_pass(**self, &begin_info, vk::SubpassContents::INLINE) };

        self
    }

    pub fn bind_pipeline(self, pipeline: &Pipeline) -> Self {
        unsafe { self.device.cmd_bind_pipeline(**self, vk::PipelineBindPoint::GRAPHICS, **pipeline) };

        self
    }

    pub fn bind_descriptor_set(
        self,
        pipeline: &Pipeline,
        descriptor_set: &Set,
    ) -> Self {
        let descriptor_sets = &[**descriptor_set];
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                **self,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.layout,
                0,
                descriptor_sets,
                &[],
            );
        }

        self
    }

    pub fn bind_index_buffer(self, index_buffer: &super::Buffer) -> Self {
        unsafe { self.device.cmd_bind_index_buffer(**self, **index_buffer, 0, vk::IndexType::UINT32) };

        self
    }

    pub fn bind_vertex_buffer(self, vertex_buffer: &super::Buffer) -> Self {
        unsafe { self.device.cmd_bind_vertex_buffers(**self, 0, &[**vertex_buffer], &[0]) };

        self
    }

    pub fn draw(self, options: DrawOptions) -> Self {
        unsafe {
            self.device.cmd_draw_indexed(
                **self,
                options.vertex_count,
                options.instance_count,
                0,
                options.first_vertex,
                options.first_instance,
            );
        };

        self
    }

    pub fn end_renderpass(self) -> Self {
        unsafe { self.device.cmd_end_render_pass(**self) };

        self
    }

    pub fn copy_buffer_to_image(self, buffer: &super::Buffer, image: &Image) -> Self {
        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D { width: image.width, height: image.height, depth: 1 });

        let regions = &[*region];
        unsafe { self.device.cmd_copy_buffer_to_image(**self, **buffer, **image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, regions) };

        self
    }

    pub fn transition_image_layout(self, image: &Image, options: &TransitionLayoutOptions) -> Self {
        let barrier = vk::ImageMemoryBarrier::builder()
            .src_access_mask(options.source_access)
            .dst_access_mask(options.destination_access)
            .old_layout(options.old)
            .new_layout(options.new)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(**image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1
            });

        let image_memory_barriers = &[*barrier];
        unsafe { self.device.cmd_pipeline_barrier(**self, options.source_stage, options.destination_stage, vk::DependencyFlags::empty(), &[], &[], image_memory_barriers) };

        self
    }

    pub fn end(self) -> Result<Buffer, vk::Result> {
        unsafe { self.device.end_command_buffer(**self)? };

        Ok(self.buffer)
    }

    pub fn submit(self) -> Result<(), vk::Result> {
        unsafe { self.device.end_command_buffer(**self)? };

        let command_buffers = &[**self];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(command_buffers);

        let submits = &[*submit_info];
        unsafe { self.device.queue_submit(*self.device.queues.graphics, submits, vk::Fence::null())? };
        unsafe { self.device.queue_wait_idle(*self.device.queues.graphics)? };

        Ok(())
    }
}

impl Deref for BufferBuilder<'_> {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl Deref for Buffer {
    type Target = vk::CommandBuffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

pub struct Pool {
    pub(crate) pool: vk::CommandPool,
    buffers: Vec<Buffer>,
}

impl Pool {
    pub fn new(device: &Device) -> Result<Self, vk::Result> {
        let create_info =
            vk::CommandPoolCreateInfo::builder().queue_family_index(device.queues.graphics.index);

        let pool = unsafe { device.create_command_pool(&create_info, None)? };

        Ok(Self {
            pool,
            buffers: Vec::new(),
        })
    }

    pub fn allocate<'a>(&'a mut self, device: &'a Device) -> Result<BufferBuilder, vk::Result> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let buffer = unsafe { device.allocate_command_buffers(&allocate_info)?[0] };
        let buffer = Buffer { buffer };
        self.buffers.push(buffer.clone());

        let builder = BufferBuilder { buffer, device };

        Ok(builder)
    }

    pub fn clear(&mut self, device: &Device) {
        if self.buffers.is_empty() {
            return;
        }

        unsafe {
            device.free_command_buffers(
                **self,
                &self
                    .buffers
                    .iter()
                    .map(|buffer| **buffer)
                    .collect::<Vec<vk::CommandBuffer>>(),
            );
        }

        self.buffers = Vec::new();
    }
}

impl Deref for Pool {
    type Target = vk::CommandPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
