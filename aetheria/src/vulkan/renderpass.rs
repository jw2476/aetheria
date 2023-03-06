use super::Device;
use ash::{prelude::*, vk};
use std::ops::Deref;

pub struct Renderpass {
    pub(crate) renderpass: vk::RenderPass,
}

impl Renderpass {
    pub fn new(device: &Device, color_format: vk::Format) -> Result<Self, vk::Result> {
        let color_attachment = vk::AttachmentDescription::builder()
            .format(color_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let color_attachments = &[*color_attachment_ref];
        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(color_attachments);

        let attachments = &[*color_attachment];
        let subpasses = &[*subpass];
        let create_info = vk::RenderPassCreateInfo::builder()
            .attachments(attachments)
            .subpasses(subpasses);

        let renderpass = unsafe { device.create_render_pass(&create_info, None)? };

        Ok(Self { renderpass })
    }
}

impl Deref for Renderpass {
    type Target = vk::RenderPass;

    fn deref(&self) -> &Self::Target {
        &self.renderpass
    }
}
