use super::{Device, Image, Renderpass, Set, graphics, compute};
use ash::vk;
use std::{ops::Deref, result::Result, sync::Arc};

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

enum Pipeline {
    Graphics(graphics::Pipeline),
    Compute(compute::Pipeline)
}

impl Pipeline {
    pub fn get_layout(&self) -> vk::PipelineLayout {
        match self {
            Pipeline::Graphics(graphics) => graphics.layout,
            Pipeline::Compute(compute) => compute.layout
        }
    }

    pub fn get_bind_point(&self) -> vk::PipelineBindPoint {
        match self {
            Pipeline::Compute(_) => vk::PipelineBindPoint::COMPUTE,
            Pipeline::Graphics(_) => vk::PipelineBindPoint::GRAPHICS
        }
    }
}

pub struct BufferBuilder {
    buffer: Buffer,
    device: Arc<Device>,
    pipeline: Option<Pipeline> 
}

#[derive(Clone, Debug)]
pub struct TransitionLayoutOptions {
    pub old: vk::ImageLayout,
    pub new: vk::ImageLayout,
    pub source_access: vk::AccessFlags,
    pub destination_access: vk::AccessFlags,
    pub source_stage: vk::PipelineStageFlags,
    pub destination_stage: vk::PipelineStageFlags,
}

impl BufferBuilder {
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

        let depth_clear_value = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };

        let clear_values = &[color_clear_value, depth_clear_value];
        let begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(**renderpass)
            .framebuffer(framebuffer)
            .render_area(*render_area)
            .clear_values(clear_values);

        unsafe {
            self.device
                .cmd_begin_render_pass(**self, &begin_info, vk::SubpassContents::INLINE)
        };

        self
    }

    pub fn bind_graphics_pipeline(mut self, pipeline: graphics::Pipeline) -> Self {
        unsafe {
            self.device
                .cmd_bind_pipeline(**self, vk::PipelineBindPoint::GRAPHICS, *pipeline)
        };

        self.pipeline = Some(Pipeline::Graphics(pipeline));

        self
    }

    pub fn bind_compute_pipeline(mut self, pipeline: compute::Pipeline) -> Self {
        unsafe {
            self.device
                .cmd_bind_pipeline(**self, vk::PipelineBindPoint::COMPUTE, *pipeline)
        };
    
        self.pipeline = Some(Pipeline::Compute(pipeline));

        self
    }

    pub fn bind_descriptor_set(
        self,
        binding: u32,
        descriptor_set: &Set,
    ) -> Self {
        let descriptor_sets = &[**descriptor_set];
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                **self,
                self.pipeline.as_ref().unwrap().get_bind_point(),
                self.pipeline.as_ref().unwrap().get_layout(),
                binding,
                descriptor_sets,
                &[],
            );
        }

        self
    }

    pub fn bind_index_buffer(self, index_buffer: &super::Buffer) -> Self {
        unsafe {
            self.device
                .cmd_bind_index_buffer(**self, **index_buffer, 0, vk::IndexType::UINT32)
        };

        self
    }

    pub fn bind_vertex_buffer(self, vertex_buffer: &super::Buffer) -> Self {
        unsafe {
            self.device
                .cmd_bind_vertex_buffers(**self, 0, &[**vertex_buffer], &[0])
        };

        self
    }

    pub fn next_subpass(self) -> Self {
        unsafe {
            self.device
                .cmd_next_subpass(**self, vk::SubpassContents::INLINE)
        };

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

    pub fn dispatch(self, x: u32, y: u32, z: u32) -> Self {
        unsafe {
            self.device.cmd_dispatch(**self, x, y, z);
        }
        
        self
    }

    pub fn copy_image(self, from: &Image, to: &Image, from_layout: vk::ImageLayout, to_layout: vk::ImageLayout, aspect: vk::ImageAspectFlags) -> Self {
        unsafe {
            let subresource = vk::ImageSubresourceLayers::builder()
                .aspect_mask(aspect)
                .mip_level(0)
                .base_array_layer(0)
                .layer_count(1);
            let copy_info = vk::ImageCopy::builder()
                .src_subresource(*subresource)
                .src_offset(vk::Offset3D::default())
                .dst_subresource(*subresource)
                .dst_offset(vk::Offset3D::default())
                .extent(vk::Extent3D { width: from.width, height: from.height, depth: 1 });
            self.device.cmd_copy_image(**self, from.image, from_layout, to.image, to_layout, &[*copy_info]);
        }
        
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
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width: image.width,
                height: image.height,
                depth: 1,
            });

        let regions = &[*region];
        unsafe {
            self.device.cmd_copy_buffer_to_image(
                **self,
                **buffer,
                **image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                regions,
            )
        };

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
                layer_count: 1,
            });

        let image_memory_barriers = &[*barrier];
        unsafe {
            self.device.cmd_pipeline_barrier(
                **self,
                options.source_stage,
                options.destination_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                image_memory_barriers,
            )
        };

        self
    }

    pub fn end(self) -> Result<Buffer, vk::Result> {
        unsafe { self.device.end_command_buffer(**self)? };

        Ok(self.buffer)
    }

    pub fn submit(self) -> Result<(), vk::Result> {
        unsafe { self.device.end_command_buffer(**self)? };

        let command_buffers = &[**self];
        let submit_info = vk::SubmitInfo::builder().command_buffers(command_buffers);

        let submits = &[*submit_info];
        unsafe {
            self.device
                .queue_submit(*self.device.queues.graphics, submits, vk::Fence::null())?
        };
        unsafe { self.device.queue_wait_idle(*self.device.queues.graphics)? };

        Ok(())
    }
}

impl Deref for BufferBuilder {
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
    device: Arc<Device>,
}

impl Pool {
    pub fn new(device: Arc<Device>) -> Result<Self, vk::Result> {
        let create_info =
            vk::CommandPoolCreateInfo::builder().queue_family_index(device.queues.graphics.index);

        let pool = unsafe { device.create_command_pool(&create_info, None)? };

        Ok(Self {
            pool,
            buffers: Vec::new(),
            device,
        })
    }

    pub fn allocate(&mut self) -> Result<BufferBuilder, vk::Result> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let buffer = unsafe { self.device.allocate_command_buffers(&allocate_info)?[0] };
        let buffer = Buffer { buffer };
        self.buffers.push(buffer.clone());

        let builder = BufferBuilder {
            buffer,
            device: self.device.clone(),
            pipeline: None
        };

        Ok(builder)
    }

    pub fn clear(&mut self) {
        if self.buffers.is_empty() {
            return;
        }

        unsafe {
            self.device.free_command_buffers(
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
