use super::{Device, Renderpass};
use ash::{prelude::*, vk};
use cstr::cstr;
use std::{ops::Deref, result::Result};

pub struct Shader {
    code: Vec<u8>,
    module: vk::ShaderModule,
    pub stage: vk::ShaderStageFlags,
}

impl Shader {
    pub fn new(
        device: &Device,
        code: Vec<u8>,
        stage: vk::ShaderStageFlags,
    ) -> Result<Self, vk::Result> {
        let (_, aligned, _) = unsafe { code.align_to::<u32>() };

        let create_info = vk::ShaderModuleCreateInfo::builder().code(aligned);

        let module = unsafe { device.create_shader_module(&create_info, None)? };

        Ok(Self {
            code,
            module,
            stage,
        })
    }

    fn get_stage(&self) -> vk::PipelineShaderStageCreateInfoBuilder {
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(self.stage)
            .module(self.module)
            .name(cstr!("main"))
    }
}

pub struct Shaders {
    pub vertex: Option<Shader>,
    pub fragment: Option<Shader>,
}

pub struct GraphicsPipeline {
    pub(crate) pipeline: vk::Pipeline,
    pub(crate) layout: vk::PipelineLayout,
    pub shaders: Shaders,
}

impl GraphicsPipeline {
    pub fn new(
        device: &Device,
        renderpass: &Renderpass,
        shaders: Shaders,
        extent: vk::Extent2D,
    ) -> Result<Self, vk::Result> {
        let vertex_stage = shaders
            .vertex
            .as_ref()
            .expect("All graphics pipleines need a vertex shader")
            .get_stage();
        let fragment_stage = shaders
            .fragment
            .as_ref()
            .expect("All graphics pipleine need a fragment shader")
            .get_stage();

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder();
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(extent);
        let viewports = &[viewport.build()];
        let scissors = &[scissor.build()];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(viewports)
            .scissors(scissors);

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false)
            .build();
        let attachments = &[attachment];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let layout_info = vk::PipelineLayoutCreateInfo::builder();
        let layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };

        let stages = &[*vertex_stage, *fragment_stage];

        let create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .layout(layout)
            .render_pass(**renderpass)
            .subpass(0);

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[*create_info], None)
                .expect("Graphics pipeline creation failed")[0]
        };

        Ok(Self {
            pipeline,
            layout,
            shaders,
        })
    }
}

impl Deref for GraphicsPipeline {
    type Target = vk::Pipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}
