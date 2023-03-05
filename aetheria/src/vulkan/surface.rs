use super::Instance;
use ash::{prelude::*, vk};
use std::{ffi::c_void, ops::Deref, result::Result};
use winit::{platform::x11::WindowExtX11, window::Window};

pub struct Surface {
    pub(crate) surface: vk::SurfaceKHR,
}

impl Surface {
    #[cfg(target_os = "linux")]
    pub fn new(instance: &Instance, window: &Window) -> Result<Self, vk::Result> {
        let create_info = vk::XlibSurfaceCreateInfoKHR::builder()
            .dpy(window.xlib_display().unwrap() as *mut *const c_void)
            .window(window.xlib_window().unwrap());

        let surface = unsafe {
            instance
                .extensions
                .xlib_surface
                .as_ref()
                .unwrap()
                .create_xlib_surface(&create_info, None)?
        };

        Ok(Self { surface })
    }
}

impl Deref for Surface {
    type Target = vk::SurfaceKHR;

    fn deref(&self) -> &Self::Target {
        &self.surface
    }
}
