#![feature(let_chains)]
#![feature(trivial_bounds)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

extern crate core;

mod macros;
mod renderer;
mod material;
mod time;
mod camera;
mod transform;
mod input;

use std::{sync::Arc, ops::Deref};
use ash::vk;
use assets::{MeshRegistry, ShaderRegistry};
use bytemuck::cast_slice;
use camera::Camera;
use time::Time;
use transform::Transform;
use vulkan::Context;
use renderer::{Renderer, Renderable, RenderObject, Light};
use winit::{event_loop::ControlFlow, event::{VirtualKeyCode, MouseButton}};
use glam::{Vec3, Quat, Vec4};
use input::{Keyboard, Mouse};

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
            .set_color(Vec3::new(0.9150942, 0.6063219, 0.4359647))
            .set_transform(transform.clone())
            .build()?;
        let foliage = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("tree.foliage.obj")?
            .set_color(Vec3::new(0.2588235, 0.7921569, 0.6034038))
            .set_transform(transform.clone())
            .build()?;

        Ok(Self {
            transform,
            trunk,
            foliage
        })
    }

    pub fn update_transform(&mut self) -> Result<(), vk::Result> {
        self.trunk.transform = self.transform.clone();
        self.foliage.transform = self.transform.clone();
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
            .set_color(Vec3::new(0.6916608, 0.8617874, 0.9339623))
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
            .set_color(Vec3::new(0.2588235, 0.7921569, 0.6034038))
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


const CAMERA_SENSITIVITY: f32 = 250.0;
const MOVEMENT_SENSITIVITY: f32 = 1.0;

struct Sun {
    noon_pos: Vec3,
    pub light: Light,
    theta: f32
}

impl Sun {
    pub fn new(noon_pos: Vec3, strength: f32, color: Vec3) -> Self {
        Self { noon_pos, light: Light::new(noon_pos, strength, color), theta: 0.0 }
    }

    pub fn update_theta(&mut self, theta: f32) {
        self.theta = theta;
        self.light.position = Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), self.theta) * self.noon_pos;
        self.light.color = Vec3::new(0.7 + 0.1 * self.theta.sin().powf(2.0), 0.2 + 0.8 * self.theta.cos().powf(2.0), 0.8 * self.theta.cos().powf(2.0));
    }

    pub fn frame_finished(&mut self, time: &Time) {
        self.update_theta(self.theta + (time.delta_seconds() * (std::f32::consts::PI / 15.0)));
    }
}

impl Deref for Sun {
    type Target = Light;

    fn deref(&self) -> &Self::Target {
        &self.light
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let (event_loop, window) = create_window();
    let window = Arc::new(window);
    let ctx = Context::new(&window);
    
    let mut mesh_registry = MeshRegistry::new();
    let mut shader_registry = ShaderRegistry::new();

    let mut renderer = Renderer::new(ctx, &mut shader_registry, window.clone()).unwrap();
    let mut camera = Camera::new(480.0, 270.0).unwrap();
    let mut time = Time::new().unwrap();
    let mut keyboard = Keyboard::new();
    let mut mouse = Mouse::new();

    let tree = Tree::load(&mut renderer, &mut mesh_registry, Transform::IDENTITY).unwrap();
    let grass = Grass::load(&mut renderer, &mut mesh_registry, Transform::IDENTITY).unwrap();


    let mut sun = Sun::new(Vec3::new(0.0, 10000.0, 0.0), 0.0, Vec3::new(0.8, 1.0, 0.5));
    sun.light.strength = sun.light.position.length().powf(2.0) * 3.5;
    event_loop.run(move |event, _, control_flow| {
        if let ControlFlow::ExitWithCode(_) = *control_flow {
            return;
        }
        
        control_flow.set_poll();

        keyboard.on_event(&event);
        mouse.on_event(&event);

        match event {
            winit::event::Event::WindowEvent { event, .. } => {     
                match event {
                    winit::event::WindowEvent::Resized(size) => {
                        renderer.recreate_swapchain().unwrap();
                        camera.width = size.width as f32;
                        camera.height = size.height as f32;
                    },
                    winit::event::WindowEvent::CloseRequested => {
                        control_flow.set_exit()
                    },
                    _ => ()     
                }

            },
            winit::event::Event::MainEventsCleared => {
                if keyboard.is_key_down(VirtualKeyCode::Escape) { control_flow.set_exit() }
                if mouse.is_button_down(MouseButton::Right) { camera.theta -= mouse.delta.x / CAMERA_SENSITIVITY }
                if keyboard.is_key_down(VirtualKeyCode::W) { camera.target -= camera.get_rotation() * Vec3::new(0.0, 0.0, MOVEMENT_SENSITIVITY) }
                if keyboard.is_key_down(VirtualKeyCode::S) { camera.target += camera.get_rotation() * Vec3::new(0.0, 0.0, MOVEMENT_SENSITIVITY) }
                if keyboard.is_key_down(VirtualKeyCode::A) { camera.target -= camera.get_rotation() * Vec3::new(MOVEMENT_SENSITIVITY, 0.0, 0.0) }
                if keyboard.is_key_down(VirtualKeyCode::D) { camera.target += camera.get_rotation() * Vec3::new(MOVEMENT_SENSITIVITY, 0.0, 0.0) }

                if keyboard.is_key_pressed(VirtualKeyCode::Up) { sun.light.strength *= 2.0; println!("Multiplier: {}", sun.light.strength / sun.light.position.length()); }
                if keyboard.is_key_pressed(VirtualKeyCode::Down) { sun.light.strength /= 2.0; println!("Multiplier: {}", sun.light.strength / sun.light.position.length()); }

                renderer.render(&[&tree, &grass], &[*sun], &camera, &time);
                time.frame_finished();
                keyboard.frame_finished();
                camera.frame_finished();
                mouse.frame_finished();
                sun.frame_finished(&time);
            }
            _ => ()
        };

        if let ControlFlow::ExitWithCode(_) = *control_flow {
            println!("Waiting for GPU to finish");
            unsafe { renderer.device.device_wait_idle().unwrap() };
        }
    });
}
