#![feature(let_chains)]
#![feature(trivial_bounds)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

extern crate core;

mod macros;
mod renderer;
mod mesh;
mod material;
mod time;
mod camera;

use std::sync::Arc;
use ash::vk;
use assets::{MeshRegistry, Mesh};
use bytemuck::cast_slice;
use camera::Camera;
use material::Material;
use time::Time;
use vulkan::Context;
use renderer::{Renderer, MeshMaterial};
use winit::event_loop::ControlFlow;
use glam::{Vec3, Quat, EulerRot, Vec4};

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


struct Tree {
    pub trunk: MeshMaterial,
    pub foliage: MeshMaterial
}

impl Tree {
    pub fn load(renderer: &mut Renderer, mesh_registry: &mut MeshRegistry) -> Result<Tree, vk::Result> {
        let trunk_color = Vec4::new(0.9150942, 0.6063219, 0.4359647, 1.0);
        let trunk = MeshMaterial::new(renderer, mesh_registry, "tree.trunk.obj", trunk_color)?;
        let foliage_color = Vec4::new(0.2588235, 0.7921569, 0.6034038, 1.0);
        let foliage = MeshMaterial::new(renderer, mesh_registry, "tree.foliage.obj", foliage_color)?;

        Ok(Self {
           trunk,
           foliage
        })
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let (event_loop, window) = create_window();
    let window = Arc::new(window);
    let ctx = Context::new(&window);
    
    let mut renderer = Renderer::new(ctx, window.clone(), &event_loop).unwrap();
    let mut camera = Camera::new(&mut renderer).unwrap();
    let mut mesh_registry = MeshRegistry::new();
    let tree = Tree::load(&mut renderer, &mut mesh_registry).unwrap();

    event_loop.run(move |event, _, control_flow| {
        if let ControlFlow::ExitWithCode(_) = *control_flow {
            return;
        }
        
        control_flow.set_poll();

        match event {
            winit::event::Event::WindowEvent { event, .. } => {     
                let egui_ctx = &renderer.egui_ctx;
                renderer.egui_winit_state.lock().on_event(egui_ctx, &event);
                           
                match event {
                    winit::event::WindowEvent::Resized(size) => {
                        renderer.recreate_swapchain().unwrap();
                        camera.update(size.width as f32, size.height as f32);
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
                    let width = renderer.ctx.swapchain.extent.width;
                    let height = renderer.ctx.swapchain.extent.height;
                    
                    let quat = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), (-delta.0 / 100.0) as f32);
                    camera.eye = quat * camera.eye;
                    camera.update(width as f32, height as f32);
                },
                _ => ()
            },
            winit::event::Event::MainEventsCleared => {
                renderer.render(&[tree.trunk.clone(), tree.foliage.clone()], &camera);
            }
            _ => ()
        };

        if let ControlFlow::ExitWithCode(_) = *control_flow {
            println!("Waiting for GPU to finish");
            unsafe { renderer.device.device_wait_idle().unwrap() };
        }
    });
}
