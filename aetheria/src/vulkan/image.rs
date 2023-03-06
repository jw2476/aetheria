use super::Device;
use ash::{prelude::*, vk};
use std::ops::Deref;

#[derive(Clone, Copy, Debug)]
pub struct Image {
    pub(crate) image: vk::Image,
    pub width: u32,
    pub height: u32,
}

impl Image {
    pub fn from_image(image: vk::Image, width: u32, height: u32) -> Self {
        Self {
            image,
            width,
            height,
        }
    }
}

impl Deref for Image {
    type Target = vk::Image;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}
