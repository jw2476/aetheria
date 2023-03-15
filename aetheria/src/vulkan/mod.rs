pub mod instance;
pub use instance::Instance;

pub mod buffer;
pub use buffer::Buffer;

pub mod command;
pub use command::DrawOptions;

pub mod context;
pub use context::Context;

pub mod descriptor;
pub use descriptor::*;

pub mod device;
pub use device::Device;

pub mod image;
pub use image::Image;

pub mod graphics;
pub use graphics::{Pipeline, Shader, Shaders};

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

fn intersection<T: Hash + Clone + Eq>(a: &[T], b: &[T]) -> Vec<T> {
    let a_unique: HashSet<T> = a.iter().cloned().collect();
    let b_unique: HashSet<T> = b.iter().cloned().collect();
    a_unique.intersection(&b_unique).cloned().collect()
}
