#![feature(cstr_from_bytes_until_nul)]
#![feature(let_chains)]

use ash::{
    vk::{self, ShaderStageFlags},
    Entry,
};

pub mod vulkan;
use vulkan::VulkanContext;

fn create_window() -> (winit::event_loop::EventLoop<()>, winit::window::Window) {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .build(&event_loop)
        .unwrap();
    (event_loop, window)
}
fn main() {
    tracing_subscriber::fmt::init();

    let (event_loop, window) = create_window();
    let mut context = VulkanContext::new(&window).unwrap();

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();

        match event {
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } => {
                control_flow.set_exit();
            }
            winit::event::Event::DeviceEvent { event: winit::event::DeviceEvent::Key(input), .. } => {
                if let Some(key) = input.virtual_keycode && key == winit::event::VirtualKeyCode::Escape {
                    control_flow.set_exit();
                }
            }
            winit::event::Event::MainEventsCleared => {
                context.render(&window);
            }
            _ => {}
        }
    })
}
