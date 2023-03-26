#![feature(let_chains)]
#![feature(trivial_bounds)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

extern crate core;

mod macros;
mod renderer;
mod mesh;

use std::path::Path;
use std::rc::Rc;

use bevy_ecs::world::World;
use bytemuck::cast_slice;
use vulkan::Context;
use renderer::Renderer;
use winit::event_loop::ControlFlow;
use glam::{Vec2, Vec3};
use crate::mesh::{Mesh, MeshRef, MeshRegistry, Texture, TextureRegistry, Transform, TransformRef, TransformRegistry, Vertex};

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
    let window = Rc::new(window);
    let ctx = Context::new(&window);
    let mut renderer = Renderer::new(ctx, window.clone()).unwrap();

    let mut world = World::new();
    world.insert_resource(MeshRegistry::new());
    world.insert_resource(TextureRegistry::new());
    world.insert_resource(TransformRegistry::new());

    let texture = Texture::new(&mut renderer, Path::new("../../assets/textures/compiled/test.qoi")).unwrap();
    let texture = world.get_resource_mut::<TextureRegistry>().unwrap().add(texture);

    let vertices = vec![
        Vertex {
            pos: Vec3::new(-0.5, -0.5, 0.0),
            uv: Vec2::new(1.0, 0.0)
        },
        Vertex {
            pos: Vec3::new(0.5, -0.5, 0.0),
            uv: Vec2::new(0.0, 0.0)
        },
        Vertex {
            pos: Vec3::new(0.5, 0.5, 0.0),
            uv: Vec2::new(0.0, 1.0)
        },
        Vertex {
            pos: Vec3::new(-0.5, 0.5, 0.0),
            uv: Vec2::new(1.0, 1.0)
        }
    ];
    let indices = vec![0, 1, 2, 2, 3, 0];
    let mesh = Mesh::new(&renderer, &vertices, &indices, Some(texture)).unwrap();
    let mesh: MeshRef = world.get_resource_mut::<MeshRegistry>().unwrap().add(mesh);

    let transform = Transform::new(&mut renderer).unwrap();
    let transform: TransformRef = world.get_resource_mut::<TransformRegistry>().unwrap().add(transform);

    let entity = world.spawn((mesh, transform));


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
            }
            winit::event::Event::DeviceEvent { event: winit::event::DeviceEvent::Key(input), .. } => {
                if let Some(key) = input.virtual_keycode && key == winit::event::VirtualKeyCode::Escape {
                    control_flow.set_exit();
                }
            }
            winit::event::Event::MainEventsCleared => {
            }
            _ => {}
        }
    });
}
