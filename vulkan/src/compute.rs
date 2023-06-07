use ash::vk::{self, DescriptorSetLayout};

use crate::{Shader, Device, SetLayout};
use std::{sync::Arc, ops::Deref};

#[derive(Clone)]
pub struct Pipeline {
    shader: Arc<Shader>,
    pub(crate) layout: vk::PipelineLayout,
    pipeline: vk::Pipeline
}

impl Pipeline {
    pub fn new(device: &Device, shader: Arc<Shader>, layouts: &[SetLayout]) -> Result<Self, vk::Result> {
        let stage = shader.get_stage();
        let descriptors = layouts.iter().map(|layout| layout.layout).collect::<Vec<DescriptorSetLayout>>();
        let layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&descriptors);
        let layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };
        
        let pipeline_info = vk::ComputePipelineCreateInfo::builder()
            .stage(*stage)
            .layout(layout);
        let pipeline = unsafe { device.create_compute_pipelines(vk::PipelineCache::null(), &[*pipeline_info], None).expect("Failed to create compute pipeline")[0] };
        Ok(Self {
            shader,
            layout,
            pipeline
        })
    }
}

impl Deref for Pipeline {
    type Target = vk::Pipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}
