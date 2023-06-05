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
mod transform;

use std::sync::Arc;
use ash::vk;
use assets::MeshRegistry;
use bytemuck::cast_slice;
use camera::Camera;
use time::Time;
use transform::Transform;
use vulkan::Context;
use renderer::{Renderer, RenderObject, Renderable};
use winit::event_loop::ControlFlow;
use glam::{Vec3, Quat, Vec4};

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
    pub transform: Transform,
    trunk: RenderObject,
    foliage: RenderObject,
}

impl Tree {
    pub fn load(renderer: &mut Renderer, mesh_registry: &mut MeshRegistry, transform: Transform) -> Result<Tree, vk::Result> {
        let trunk = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("tree.trunk.obj")?
            .set_color(Vec4::new(0.9150942, 0.6063219, 0.4359647, 1.0))
            .set_transform(transform.clone())
            .build()?;
        let foliage = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("tree.foliage.obj")?
            .set_color(Vec4::new(0.2588235, 0.7921569, 0.6034038, 1.0))
            .set_transform(transform.clone())
            .build()?;

        Ok(Self {
            transform,
            trunk,
            foliage
        })
    }

    pub fn update_transform(&mut self) -> Result<(), vk::Result> {
        let mut trunk = self.trunk.transform.lock();
        let mut foliage = self.foliage.transform.lock();
        trunk.transform = self.transform.clone();
        trunk.update()?;
        foliage.transform = self.transform.clone();
        foliage.update()?;
        Ok(())
    }
}

impl Renderable for Tree {
    fn get_objects(&self) -> Vec<&RenderObject> {
        vec![&self.trunk, &self.foliage]
    }
}

struct Rock {
    pub transform: Transform,
    rock: RenderObject
}

impl Rock {
    pub fn load(renderer: &mut Renderer, mesh_registry: &mut MeshRegistry, transform: Transform) -> Result<Self, vk::Result> {
        let rock = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("rocks.obj")?
            .set_color(Vec4::new(0.6916608, 0.8617874, 0.9339623, 1.0))
            .set_transform(transform.clone())
            .build()?;
        Ok(Self { transform, rock })
    }
}

impl Renderable for Rock {
    fn get_objects(&self) -> Vec<&RenderObject> {
        vec![&self.rock]
    }
}

struct Grass {
    pub transform: Transform,
    grass: RenderObject
}

impl Grass {
    pub fn load(renderer: &mut Renderer, mesh_registry: &mut MeshRegistry, transform: Transform) -> Result<Self, vk::Result> {
        let grass = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("grass.obj")?
            .set_color(Vec4::new(0.2588235, 0.7921569, 0.6034038, 1.0))
            .set_transform(transform.clone())
            .build()?;
        Ok(Self { transform, grass })
    }
}

impl Renderable for Grass {
    fn get_objects(&self) -> Vec<&RenderObject> {
        vec![&self.grass]
    }
}

fn get_coord() -> f32 {
    (rand::random::<f32>() - 0.5) * 25.0
}

fn main() {
    tracing_subscriber::fmt::init();

    let (event_loop, window) = create_window();
    let window = Arc::new(window);
    let ctx = Context::new(&window);
    
    let mut renderer = Renderer::new(ctx, window.clone(), &event_loop).unwrap();
    let mut camera = Camera::new(&renderer).unwrap();
    let mut time = Time::new(&renderer).unwrap();
    let mut mesh_registry = MeshRegistry::new();

    let mut renderables = Vec::new();
    for _ in 0..100 {
        let mut transform = Transform::IDENTITY;
        transform.translation = Vec3::new(get_coord(), 0.0, get_coord());
        let tree = Tree::load(&mut renderer, &mut mesh_registry, transform).unwrap();
        let renderable: Box<dyn Renderable> = Box::new(tree);
        renderables.push(renderable);
    }

    for _ in 0..100 {
        let mut transform = Transform::IDENTITY;
        transform.translation = Vec3::new(get_coord(), 0.0, get_coord());
        transform.scale *= 0.6;
        let rock = Rock::load(&mut renderer, &mut mesh_registry, transform).unwrap();
        let renderable: Box<dyn Renderable> = Box::new(rock);
        renderables.push(renderable);
    }
    
    let grass = Grass::load(&mut renderer, &mut mesh_registry, Transform::IDENTITY).unwrap();

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
                renderer.render(&renderables, &grass, &camera);
                time.frame_finished();
            }
            _ => ()
        };

        if let ControlFlow::ExitWithCode(_) = *control_flow {
            println!("Waiting for GPU to finish");
            unsafe { renderer.device.device_wait_idle().unwrap() };
        }
    });
}
