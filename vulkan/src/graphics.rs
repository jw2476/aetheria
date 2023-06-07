use super::{Device, Renderpass, SetLayout};
use ash::vk;
use bytemuck::cast_slice;
use cstr::cstr;
use std::{ops::Deref, result::Result};

#[derive(Clone)]
pub struct Shader {
    module: vk::ShaderModule,
    pub stage: vk::ShaderStageFlags,
}

impl Shader {
    pub fn new(
        device: &ash::Device,
        code: &[u8],
        stage: vk::ShaderStageFlags,
    ) -> Result<Self, vk::Result> {
        let create_info = vk::ShaderModuleCreateInfo::builder().code(cast_slice(code));

        let module = unsafe { device.create_shader_module(&create_info, None)? };

        Ok(Self { module, stage })
    }

    pub fn get_stage(&self) -> vk::PipelineShaderStageCreateInfoBuilder {
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(self.stage)
            .module(self.module)
            .name(cstr!("main"))
    }
}

#[derive(Clone)]
pub struct Shaders {
    pub vertex: Option<Shader>,
    pub fragment: Option<Shader>,
}

pub struct Binding {
    binding: usize,
    stride: usize,
    attributes: Vec<vk::VertexInputAttributeDescription>,
}

impl Binding {
    pub fn add_attribute(mut self, format: vk::Format) -> Self {
        let attribute = vk::VertexInputAttributeDescription::builder()
            .binding(self.binding.try_into().unwrap())
            .location(self.attributes.len().try_into().unwrap())
            .format(format)
            .offset(self.stride.try_into().unwrap())
            .build();
        self.attributes.push(attribute);
        self.stride += match format {
            vk::Format::R32G32_SFLOAT => 2 * 4,
            vk::Format::R32G32B32_SFLOAT => 3 * 4,
            vk::Format::R32G32B32A32_SFLOAT => 4 * 4,
            vk::Format::R8G8B8A8_UINT => 4 * 1,
            _ => todo!(),
        };

        self
    }
}

pub struct VertexInputBuilder {
    bindings: Vec<Binding>,
}

impl VertexInputBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_binding<F: Fn(Binding) -> Binding>(mut self, callback: F) -> Self {
        let binding = Binding {
            binding: self.bindings.len(),
            stride: 0,
            attributes: Vec::new(),
        };
        self.bindings.push(callback(binding));

        self
    }

    fn to_vertex_bindings(
        &self,
    ) -> (
        Vec<vk::VertexInputBindingDescription>,
        Vec<vk::VertexInputAttributeDescription>,
    ) {
        let bindings = self
            .bindings
            .iter()
            .enumerate()
            .map(|(i, binding)| {
                vk::VertexInputBindingDescription::builder()
                    .binding(i.try_into().unwrap())
                    .stride(binding.stride.try_into().unwrap())
                    .input_rate(vk::VertexInputRate::VERTEX)
                    .build()
            })
            .collect::<Vec<vk::VertexInputBindingDescription>>();
        let attributes = self
            .bindings
            .iter()
            .flat_map(|binding| binding.attributes.clone())
            .collect::<Vec<vk::VertexInputAttributeDescription>>();

        (bindings, attributes)
    }
}

impl Default for VertexInputBuilder {
    fn default() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }
}

pub struct Pipeline {
    pub(crate) pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub shaders: Shaders,
}

impl Pipeline {
    pub fn new(
        device: &Device,
        renderpass: &Renderpass,
        shaders: Shaders,
        extent: vk::Extent2D,
        descriptor_layouts: &[SetLayout],
        vertex_input: VertexInputBuilder,
        subpass: u32,
        depth: bool,
        cull: bool,
    ) -> Result<Self, vk::Result> {
        let vertex_stage = shaders
            .vertex
            .as_ref()
            .expect("All graphics pipelines need a vertex shader")
            .get_stage();
        let fragment_stage = shaders
            .fragment
            .as_ref()
            .expect("All graphics pipelines need a fragment shader")
            .get_stage();

        let (bindings, attributes) = vertex_input.to_vertex_bindings();
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attributes);
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        #[allow(clippy::cast_precision_loss)]
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
            .cull_mode(if cull {
                vk::CullModeFlags::BACK
            } else {
                vk::CullModeFlags::NONE
            })
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

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

        let set_layouts: Vec<vk::DescriptorSetLayout> =
            descriptor_layouts.iter().map(|layout| **layout).collect();
        let layout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(&set_layouts);
        let layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };

        let stages = &[*vertex_stage, *fragment_stage];

        let mut create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .layout(layout)
            .render_pass(**renderpass)
            .subpass(subpass);

        if depth {
            create_info = create_info.depth_stencil_state(&depth_stencil);
        }

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

impl Deref for Pipeline {
    type Target = vk::Pipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}
