use super::Instance;
use ash::vk;
use std::{ffi::c_void, ops::Deref, result::Result};
use winit::window::Window;

#[cfg(target_os = "linux")]
use winit::platform::x11::WindowExtX11;

#[cfg(target_os = "windows")]
use winit::platform::windows::WindowExtWindows;

pub struct Surface {
    pub(crate) surface: vk::SurfaceKHR,
}

impl Surface {
    #[cfg(target_os = "linux")]
    pub fn new(instance: &Instance, window: &Window) -> Result<Self, vk::Result> {
        let create_info = vk::XlibSurfaceCreateInfoKHR::builder()
            .dpy(window.xlib_display().unwrap().cast::<*const c_void>())
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

    #[cfg(target_os = "windows")]
    pub fn new(instance: &Instance, window: &Window) -> Result<Self, vk::Result> {
        let create_info = vk::Win32SurfaceCreateInfoKHR::builder()
            .hinstance(window.hinstance() as *const c_void)
            .hwnd(window.hwnd() as *const c_void);

        let surface = unsafe {
            instance
                .extensions
                .win32_surface
                .as_ref()
                .unwrap()
                .create_win32_surface(&create_info, None)?
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
