use ash::vk;
use assets::{ShaderRegistry, TextureRegistry};
use bytemuck::{cast_slice, Pod, Zeroable};
use glam::{UVec2, Vec4};
use std::sync::Arc;
use vulkan::{
    command, command::TransitionLayoutOptions, compute, Buffer, Image, Pool, Set, SetLayout,
    SetLayoutBuilder, Shader, Texture,
};

use crate::renderer::{Pass, Renderer, RENDER_HEIGHT, RENDER_WIDTH};

pub const CHAR_WIDTH: u32 = 6;
pub const CHAR_HEIGHT: u32 = 5;

#[derive(Clone, Debug, PartialEq)]
pub struct SizeConstraints {
    pub min: UVec2,
    pub max: UVec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub origin: UVec2,
    pub size: UVec2,
}

pub trait Element: Clone + std::fmt::Debug {
    fn layout(&mut self, constraint: SizeConstraints) -> UVec2;
    fn paint(&mut self, region: Region, scene: &mut Vec<Rectangle>);
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Rectangle {
    pub color: Vec4,
    pub origin: UVec2,
    pub extent: UVec2,
    pub radius: u32,
    pub atlas_id: i32,
    pub _padding: [u8; 8],
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            color: Vec4::ONE,
            origin: UVec2::ZERO,
            extent: UVec2::ONE,
            radius: 0,
            atlas_id: -1,
            _padding: [0_u8; 8],
        }
    }
}

pub struct UIPass {
    pipeline: compute::Pipeline,
    font: Arc<Texture>,
    ui_layout: SetLayout,
    ui_pool: Pool,
    ui_set: Set,
    output: Texture,
}

impl UIPass {
    pub fn new(
        renderer: &mut Renderer,
        shader_registry: &mut ShaderRegistry,
        texture_registry: &mut TextureRegistry,
        input: &Texture,
    ) -> Result<Self, vk::Result> {
        let image = Image::new(
            &renderer,
            RENDER_WIDTH,
            RENDER_HEIGHT,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC,
        )?;
        let output = Texture::from_image(
            &renderer,
            image,
            vk::Filter::NEAREST,
            vk::Filter::NEAREST,
            true,
        )?;

        let ui_layout = SetLayoutBuilder::new(&renderer.device)
            .add(vk::DescriptorType::STORAGE_IMAGE)
            .add(vk::DescriptorType::STORAGE_IMAGE)
            .add(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .add(vk::DescriptorType::STORAGE_BUFFER)
            .build()?;
        let mut ui_pool = Pool::new(renderer.device.clone(), ui_layout.clone(), 1)?;
        let ui_set = ui_pool.allocate()?;
        ui_set.update_texture(&renderer.device, 0, &output, vk::ImageLayout::GENERAL);
        ui_set.update_texture(&renderer.device, 1, &input, vk::ImageLayout::GENERAL);

        let font = texture_registry.load(renderer, "font.qoi", false);
        ui_set.update_texture(
            &renderer.device,
            2,
            &font,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        let shader: Arc<Shader> = shader_registry.load(&renderer.device, "ui.comp.glsl");
        let pipeline =
            compute::Pipeline::new(&renderer.device, shader.clone(), &[ui_layout.clone()])?;

        Ok(Self {
            pipeline,
            ui_layout,
            ui_pool,
            ui_set,
            font,
            output,
        })
    }

    pub fn set_geometry(
        &self,
        renderer: &Renderer,
        rectangles: &[Rectangle],
    ) -> Result<(), vk::Result> {
        let mut rectangle_data: Vec<u8> =
            cast_slice::<i32, u8>(&[rectangles.len() as i32, 0, 0, 0]).to_vec();
        rectangle_data.extend_from_slice(cast_slice::<Rectangle, u8>(rectangles));
        let rectangle_buffer = Buffer::new(
            renderer,
            rectangle_data,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )?;
        self.ui_set
            .update_buffer(&renderer.device, 3, &rectangle_buffer);

        Ok(())
    }

    pub fn get_texture(&self) -> &'_ Texture {
        &self.output
    }
}

impl Pass for UIPass {
    fn record(&self, cmd: command::BufferBuilder) -> command::BufferBuilder {
        cmd.transition_image_layout(
            &self.output.image,
            &TransitionLayoutOptions {
                old: vk::ImageLayout::UNDEFINED,
                new: vk::ImageLayout::GENERAL,
                source_access: vk::AccessFlags::NONE,
                destination_access: vk::AccessFlags::SHADER_WRITE,
                source_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
                destination_stage: vk::PipelineStageFlags::COMPUTE_SHADER,
            },
        )
        .bind_compute_pipeline(self.pipeline.clone())
        .bind_descriptor_set(0, &self.ui_set)
        .dispatch(
            RENDER_WIDTH / 16,
            (RENDER_HEIGHT as f32 / 16.0).ceil() as u32,
            1,
        )
    }
}
