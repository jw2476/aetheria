use super::Device;
use ash::vk;
use std::ops::Deref;

#[derive(Clone, Copy, Debug)]
pub struct Image {
    pub(crate) image: vk::Image,
    pub format: vk::Format,
    pub width: u32,
    pub height: u32,
}

impl Image {
    pub const fn from_image(image: vk::Image, format: vk::Format, width: u32, height: u32) -> Self {
        Self {
            image,
            format,
            width,
            height,
        }
    }

    pub fn create_view_without_context(
        &self,
        device: &Device,
    ) -> Result<vk::ImageView, vk::Result> {
        let create_info = vk::ImageViewCreateInfo::builder()
            .image(**self)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(self.format)
            .components(vk::ComponentMapping::default())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        unsafe { device.create_image_view(&create_info, None) }
    }
}

impl Deref for Image {
    type Target = vk::Image;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}
