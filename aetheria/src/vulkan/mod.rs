pub mod instance;
pub use instance::Instance;

pub mod command;
pub use command::{CommandBuffer, CommandPool};

pub mod context;
pub use context::VulkanContext;

pub mod device;
pub use device::Device;

pub mod image;
pub use image::Image;

pub mod graphics;
pub use graphics::{GraphicsPipeline, Shader};

pub mod renderpass;
pub use renderpass::Renderpass;

pub mod surface;
pub use surface::Surface;

pub mod swapchain;
pub use swapchain::Swapchain;

use cstr::cstr;
use std::{clone::Clone, cmp::Eq, collections::HashSet, ffi::CStr, hash::Hash};

#[cfg(debug_assertions)]
fn get_wanted_layers() -> Vec<&'static CStr> {
    vec![cstr!("VK_LAYER_KHRONOS_validation")]
}

#[cfg(not(debug_assertions))]
fn get_wanted_layers() -> Vec<&'static CStr> {
    vec![]
}

fn intersection<T: Hash + Clone + Eq>(a: &Vec<T>, b: &Vec<T>) -> Vec<T> {
    let a_unique: HashSet<T> = HashSet::from_iter(a.iter().cloned());
    let b_unique: HashSet<T> = HashSet::from_iter(b.iter().cloned());
    a_unique
        .intersection(&b_unique)
        .cloned()
        .into_iter()
        .collect()
}
