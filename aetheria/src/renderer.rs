use ash::vk;
use assets::{Mesh, MeshRegistry, ShaderRegistry, Vertex};
use bytemuck::{cast_slice, cast_slice_mut, Pod, Zeroable};
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::Mutex;
use std::{ops::Deref, sync::Arc};
use tracing::info;
use vulkan::command::{self, TransitionLayoutOptions};
use vulkan::VertexInputBuilder;
use vulkan::{
    compute, graphics, Buffer, Context, Image, Pool, Set, SetLayout, SetLayoutBuilder, Shader,
    Swapchain, Texture,
};
use winit::window::Window;

use crate::camera::Camera;
use crate::render::Renderable;
use crate::time::Time;
use crate::transform::Transform;

pub trait Pass {
    fn record(&self, cmd: command::BufferBuilder) -> command::BufferBuilder;
}

pub struct Renderer {
    pub(crate) ctx: Context,
    window: Arc<Window>,

    render_finished: vk::Semaphore,
    in_flight: vk::Fence,
    output_image: Option<(Arc<Image>, vk::ImageLayout)>,

    passes: Vec<Arc<Mutex<dyn Pass>>>,
}

pub const RENDER_WIDTH: u32 = 480;
pub const RENDER_HEIGHT: u32 = 270;

impl Renderer {
    pub fn new(ctx: Context, window: Arc<Window>) -> Result<Self, vk::Result> {
        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let render_finished =
            unsafe { ctx.device.create_semaphore(&semaphore_info, None).unwrap() };
        let in_flight = unsafe { ctx.device.create_fence(&fence_info, None).unwrap() };

        let renderer = Self {
            ctx,
            window,
            render_finished,
            in_flight,
            output_image: None,
            passes: Vec::new(),
        };

        Ok(renderer)
    }

    unsafe fn destroy_swapchain(&mut self) {
        self.ctx.device.device_wait_idle().unwrap();

        self.ctx
            .swapchain
            .image_views
            .iter()
            .for_each(|view| self.ctx.device.destroy_image_view(*view, None));
        self.ctx
            .device
            .extensions
            .swapchain
            .as_ref()
            .unwrap()
            .destroy_swapchain(*self.ctx.swapchain, None);
    }

    pub fn recreate_swapchain(&mut self) -> Result<(), vk::Result> {
        unsafe { self.destroy_swapchain() };

        info!("Recreating swapchain");

        self.ctx.swapchain = Swapchain::new(
            &self.ctx.instance,
            &self.ctx.surface,
            &self.ctx.device,
            &self.window,
        )?;

        Ok(())
    }

    pub fn add_pass(&mut self, pass: Arc<Mutex<dyn Pass>>) {
        self.passes.push(pass);
    }

    pub fn set_output_image(&mut self, image: Arc<Image>, layout: vk::ImageLayout) {
        self.output_image = Some((image, layout));
    }

    pub fn wait_for_frame(&self) {
        unsafe {
            self.device
                .wait_for_fences(&[self.in_flight], true, u64::MAX)
                .unwrap();
        }
    }

    pub fn render(&mut self) {
        unsafe {
            let in_flight = self.in_flight.clone();

            let acquire_result = self.ctx.start_frame(in_flight);

            /*self.render_pass
                .set_geometry(&self, mesh_registry, renderables, lights);
            self.ui_pass.set_geometry(&self, &[Rectangle { origin: Vec2::new(50.0, 50.0), extent: Vec2::new(50.0, 50.0), radius: 25.0, color: Vec4::new(1.0, 0.0, 1.0, 0.3), ..Default::default() }]).expect("Failed to update UI geometry");*/

            let image_index = match acquire_result {
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.recreate_swapchain()
                        .expect("Swapchain recreation failed");
                    return;
                }
                Err(e) => panic!("{}", e),
                Ok(image_index) => image_index,
            };

            self.command_pool.clear();

            let cmd = self
                .command_pool
                .allocate()
                .unwrap()
                .begin()
                .unwrap()
                .record(|cmd| {
                    self.passes
                        .iter()
                        .fold(cmd, |cmd, pass| cmd.record(|cmd| pass.lock().unwrap().record(cmd)))
                })
                .transition_image_layout(
                    &self.output_image.as_ref().expect("No output image set").0,
                    &TransitionLayoutOptions {
                        old: self.output_image.as_ref().unwrap().1,
                        new: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                        source_access: vk::AccessFlags::SHADER_WRITE,
                        destination_access: vk::AccessFlags::TRANSFER_READ,
                        source_stage: vk::PipelineStageFlags::COMPUTE_SHADER,
                        destination_stage: vk::PipelineStageFlags::TRANSFER,
                    },
                )
                .transition_image_layout(
                    &self.ctx.swapchain.images[image_index as usize],
                    &TransitionLayoutOptions {
                        old: vk::ImageLayout::UNDEFINED,
                        new: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        source_access: vk::AccessFlags::NONE,
                        destination_access: vk::AccessFlags::TRANSFER_WRITE,
                        source_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                        destination_stage: vk::PipelineStageFlags::TRANSFER,
                    },
                )
                .blit_image(
                    &self.output_image.as_ref().unwrap().0,
                    &self.ctx.swapchain.images[image_index as usize],
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageAspectFlags::COLOR,
                    vk::Filter::NEAREST,
                )
                .transition_image_layout(
                    &self.ctx.swapchain.images[image_index as usize],
                    &TransitionLayoutOptions {
                        old: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        new: vk::ImageLayout::PRESENT_SRC_KHR,
                        source_access: vk::AccessFlags::TRANSFER_WRITE,
                        destination_access: vk::AccessFlags::NONE,
                        source_stage: vk::PipelineStageFlags::TRANSFER,
                        destination_stage: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    },
                )
                .end()
                .unwrap();

            let wait_semaphores = &[self.ctx.image_available];
            let signal_semaphores = &[self.render_finished];
            let command_buffers = &[*cmd];
            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(wait_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .command_buffers(command_buffers)
                .signal_semaphores(signal_semaphores);

            self.ctx
                .device
                .queue_submit(
                    self.ctx.device.queues.graphics.queue,
                    &[*submit_info],
                    self.in_flight,
                )
                .unwrap();

            let presentation_result = self.ctx.end_frame(image_index, self.render_finished);

            match presentation_result {
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self
                    .recreate_swapchain()
                    .expect("Swapchain recreation failed"),
                Err(e) => panic!("{}", e),
                Ok(_) => (),
            }
        }
    }
}

impl Deref for Renderer {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl DerefMut for Renderer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}
