#![feature(let_chains)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod vulkan;
use ash::vk;
use bytemuck::cast_slice;
use nalgebra::{Vector2, Vector3};
use vulkan::{Buffer, VulkanContext};
use winit::event_loop::ControlFlow;

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
    let mut ctx = VulkanContext::new(&window).unwrap();

    let positions = [
        Vector2::new(0.0, -0.5),
        Vector2::new(0.5, 0.5),
        Vector2::new(-0.5, 0.5),
    ];
    let colors = [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    ];

    let vertices: Vec<u8> = std::iter::zip(positions, colors)
        .map(|(position, color)| {
            let mut vertex: Vec<u8> = cast_slice::<f32, u8>(position.as_ref()).to_vec();
            vertex.extend_from_slice(cast_slice::<f32, u8>(color.as_ref()));
            vertex
        })
        .flatten()
        .collect();

    let mut vertex_buffer =
        Buffer::new(&ctx, &vertices, vk::BufferUsageFlags::VERTEX_BUFFER).unwrap();

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();

        match event {
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } => {
                control_flow.set_exit();
            }
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::Resized(_),
                ..
            } => {
                ctx.recreate_swapchain(&window);
            }
            winit::event::Event::DeviceEvent { event: winit::event::DeviceEvent::Key(input), .. } => {
                if let Some(key) = input.virtual_keycode && key == winit::event::VirtualKeyCode::Escape {
                    control_flow.set_exit();
                }
            }
            winit::event::Event::MainEventsCleared => {
                ctx.render(&window, &vertex_buffer);
            }
            _ => {}
        }

        if *control_flow == ControlFlow::Exit {
            unsafe { ctx.device.device_wait_idle().unwrap() };
        }
        
    });
}
