#![feature(let_chains)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

mod vulkan;
mod renderer;

use ash::vk;
use bytemuck::cast_slice;
use vulkan::{Buffer, VulkanContext};
use renderer::Renderer;
use winit::event_loop::ControlFlow;
use glam::{Vec2, Vec3};

struct Indices(Vec<u32>);
impl From<Indices> for Vec<u8> {
    fn from(indices: Indices) -> Self {
        cast_slice::<u32, u8>(&indices.0).to_vec()
    }
}

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
    let ctx = VulkanContext::new(&window).unwrap();
    let mut renderer = Renderer::new(ctx).unwrap();

    let positions = [
        Vec2::new(-0.5, -0.5),
        Vec2::new(0.5, -0.5),
        Vec2::new(-0.5, 0.5),
        Vec2::new(0.5, 0.5)
    ];
    let colors = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0)
    ];

    let vertices: Vec<u8> = std::iter::zip(positions, colors)
        .flat_map(|(position, color)| {
            let mut vertex: Vec<u8> = cast_slice::<f32, u8>(position.as_ref()).to_vec();
            vertex.extend_from_slice(cast_slice::<f32, u8>(color.as_ref()));
            vertex
        })
        .collect();

    let vertex_buffer =
        Buffer::new(&renderer, vertices, vk::BufferUsageFlags::VERTEX_BUFFER).unwrap();

    let indices = Indices(vec![0, 1, 2, 2, 1, 3]);
    let index_buffer = Buffer::new(&renderer, indices, vk::BufferUsageFlags::INDEX_BUFFER).expect("Index buffer creation failed");

    event_loop.run(move |event, _, control_flow| {
        if let ControlFlow::ExitWithCode(_) = *control_flow {
            return;
        }
        
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
                renderer.recreate_swapchain(&window);
            }
            winit::event::Event::DeviceEvent { event: winit::event::DeviceEvent::Key(input), .. } => {
                if let Some(key) = input.virtual_keycode && key == winit::event::VirtualKeyCode::Escape {
                    control_flow.set_exit();
                }
            }
            winit::event::Event::MainEventsCleared => {
                renderer.render(&window, &vertex_buffer, &index_buffer);
            }
            _ => {}
        }

        if let ControlFlow::ExitWithCode(_) = *control_flow {
            println!("Waiting for GPU to finish jobs");
            unsafe { renderer.device.device_wait_idle().unwrap() };
            println!("GPU finished");
        }
    });
}
