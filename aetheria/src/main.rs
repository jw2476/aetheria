#![feature(let_chains)]
#![feature(trivial_bounds)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

extern crate core;

mod macros;
mod renderer;
mod mesh;
mod model;
mod time;
mod camera;

use std::sync::Arc;
use bevy_ecs::{world::World, system::{Res, Query, ResMut}, schedule::Schedule};
use bytemuck::cast_slice;
use camera::Camera;
use mesh::{TextureRef, MaterialRegistry, EguiTextureRegistry};
use time::Time;
use vulkan::{Context, Texture};
use renderer::Renderer;
use winit::event_loop::ControlFlow;
use glam::{Vec2, Vec3, Quat, EulerRot};
use crate::{mesh::{Mesh, MeshRef, MeshRegistry, TextureRegistry, Transform, TransformRef, TransformRegistry, Vertex}, model::Model};

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
    
    let mut renderer = Renderer::new(ctx, window.clone(), &event_loop).unwrap();

    let mut world = World::new();
    world.insert_resource(MeshRegistry::new());
    world.insert_resource(TextureRegistry::new());
    world.insert_resource(TransformRegistry::new());
    world.insert_resource(MaterialRegistry::new());
    world.insert_resource(EguiTextureRegistry::new());
    world.insert_resource(Time::new());
    world.insert_resource(Camera::new(&mut renderer).unwrap());
    world.insert_resource(renderer);

    let white = Texture::new(&mut world.get_resource_mut::<Renderer>().unwrap().ctx, include_bytes!("../../assets/textures/compiled/white.qoi")).unwrap();
    world.get_resource_mut::<TextureRegistry>().unwrap().add(white);

    let mut schedule = Schedule::default();
    schedule.add_system(Time::frame_finished);
    schedule.add_system(Renderer::render);
    schedule.add_system(animate);
        
    Model::load(include_bytes!("../../assets/models/fence.glb"), &mut world);
    Model::load(include_bytes!("../../assets/models/tree.glb"), &mut world);
    Model::load(include_bytes!("../../assets/models/stones.glb"), &mut world);

    event_loop.run(move |event, _, control_flow| {
        if let ControlFlow::ExitWithCode(_) = *control_flow {
            return;
        }
        
        control_flow.set_poll();

        match event {
            winit::event::Event::WindowEvent { event, .. } => {     
                let egui_ctx = &world.get_resource::<Renderer>().unwrap().egui_ctx;
                world.get_resource::<Renderer>().unwrap().egui_winit_state.lock().on_event(egui_ctx, &event);
                           
                match event {
                    winit::event::WindowEvent::Resized(size) => {
                        world.get_resource_mut::<Renderer>().unwrap().recreate_swapchain().unwrap();
                        world.get_resource_mut::<Camera>().unwrap().update(size.width as f32, size.height as f32);
                    },
                    winit::event::WindowEvent::CloseRequested => {
                        control_flow.set_exit()
                    },
                    _ => ()     
                }

            },
            winit::event::Event::DeviceEvent {event, ..} => match event {
                winit::event::DeviceEvent::Key(input) => {
                    if let Some(key) = input.virtual_keycode && key == winit::event::VirtualKeyCode::Escape {
                        control_flow.set_exit()
                    }
                },
                winit::event::DeviceEvent::MouseMotion { delta } => {
                    let width;
                    let height;
                    {
                        let renderer = world.get_resource::<Renderer>().unwrap();
                        width = renderer.ctx.swapchain.extent.width;
                        height = renderer.ctx.swapchain.extent.height;
                    }

                    let quat = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), (-delta.0 / 100.0) as f32);
                    let mut camera = world.get_resource_mut::<Camera>().unwrap();
                    camera.eye = quat * camera.eye;
                    camera.update(width as f32, height as f32);
                },
                _ => ()
            },
            winit::event::Event::MainEventsCleared => {
                schedule.run(&mut world);
            }
            _ => ()
        };

        if let ControlFlow::ExitWithCode(_) = *control_flow {
            println!("Waiting for GPU to finish");
            unsafe { world.get_resource::<Renderer>().unwrap().device.device_wait_idle().unwrap() };
        }
    });
}

fn animate(time: Res<Time>, renderer: Res<Renderer>, mut registry: ResMut<TransformRegistry>) {
    registry.registry.values_mut().for_each(|transform| { 
        let mut euler = transform.rotation.to_euler(EulerRot::ZXY);
        euler.2 += time.delta_seconds() / 4.0;
        transform.rotation = Quat::from_euler(glam::EulerRot::ZXY, euler.0, euler.1, euler.2);
        transform.update(&renderer).unwrap();
    })
}
