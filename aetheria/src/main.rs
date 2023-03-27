#![feature(let_chains)]
#![feature(trivial_bounds)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

extern crate core;

mod macros;
mod renderer;
mod mesh;
mod time;

use std::path::Path;
use std::sync::Arc;

use bevy_ecs::{world::World, system::{Res, Query, ResMut}, schedule::Schedule};
use bytemuck::cast_slice;
use time::Time;
use vulkan::Context;
use renderer::Renderer;
use winit::event_loop::ControlFlow;
use glam::{Vec2, Vec3, Quat};
use crate::mesh::{Mesh, MeshRef, MeshRegistry, Texture, TextureRegistry, Transform, TransformRef, TransformRegistry, Vertex};
use gltf::Glb;

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
    let window = Arc::new(window);
    let ctx = Context::new(&window);
    let mut renderer = Renderer::new(ctx, window.clone()).unwrap();

    let mut world = World::new();
    world.insert_resource(MeshRegistry::new());
    world.insert_resource(TextureRegistry::new());
    world.insert_resource(TransformRegistry::new());
    world.insert_resource(Time::new());

    let mut schedule = Schedule::default();
    schedule.add_system(animate);
    schedule.add_system(Time::frame_finished);
    schedule.add_system(Renderer::render);
    
    let texture = Texture::new(&mut renderer, Path::new("../../assets/textures/compiled/test.qoi")).unwrap();
    let texture = world.get_resource_mut::<TextureRegistry>().unwrap().add(texture);
    
    let model = Glb::load(include_bytes!("../../assets/models/samples/2.0/Duck/glTF-Binary/Duck.glb")).unwrap();
    let meshes = model.gltf.meshes.iter().flat_map(|mesh| mesh.primitives.clone()).map(|primitive| {
        let positions: Vec<Vec3> = cast_slice::<u8, f32>(&primitive.get_attribute_data(&model, "POSITION").unwrap())
            .chunks_exact(3)
            .map(|pos| Vec3::from_slice(pos))
            .collect();
        let uvs: Vec<Vec2> = cast_slice::<u8, f32>(&primitive.get_attribute_data(&model, "TEXCOORD_0").unwrap())
            .chunks_exact(2)
            .map(|uv| Vec2::from_slice(uv))
            .collect();

        let vertices = std::iter::zip(positions, uvs).map(|(pos, uv)| Vertex { pos, uv }).collect::<Vec<Vertex>>();
        let indices = primitive.get_indices_data(&model).unwrap();

        let mesh = Mesh::new(&renderer.ctx, &vertices, &indices, Some(texture)).unwrap();
        let mesh: MeshRef = world.get_resource_mut::<MeshRegistry>().unwrap().add(mesh);
        mesh
    }).collect::<Vec<MeshRef>>();
    println!("{:?}", model.gltf);

    let mut transform = Transform::new(&mut renderer).unwrap();
    transform.scale = Vec3::new(0.005, 0.005, 0.005);
    transform.update(&renderer).unwrap();
    let transform: TransformRef = world.get_resource_mut::<TransformRegistry>().unwrap().add(transform);

    meshes.iter().for_each(|mesh| {
        world.spawn((*mesh, transform));
    });
    
    world.insert_resource(renderer);

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
                schedule.run(&mut world);
            }
            _ => {}
        }

        if let ControlFlow::ExitWithCode(_) = *control_flow {
            println!("Waiting for GPU to finish");
            unsafe { world.get_resource::<Renderer>().unwrap().device.device_wait_idle().unwrap() };
        }
    });
}

fn animate(time: Res<Time>, renderer: Res<Renderer>, mut registry: ResMut<TransformRegistry>, query: Query<&TransformRef>) {
    for &transform in query.iter() {
        let transform = registry.get_mut(transform).unwrap();
        let mut euler = transform.rotation.to_euler(glam::EulerRot::XYZ);
        euler.2 += time.delta_seconds();
        println!("{:?}", euler);
        transform.rotation = Quat::from_euler(glam::EulerRot::XYZ, euler.0, euler.1, euler.2);
        transform.update(&renderer);
    }
}