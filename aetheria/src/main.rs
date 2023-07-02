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

use std::{sync::Arc, ops::Deref, time::Instant, f32::consts::PI};
use ash::vk;
use assets::{MeshRegistry, ShaderRegistry};
use bytemuck::cast_slice;
use camera::Camera;
use rand::Rng;
use time::Time;
use transform::Transform;
use vulkan::Context;
use renderer::{Renderer, Renderable, RenderObject, Light};
use winit::{event_loop::ControlFlow, event::{VirtualKeyCode, MouseButton}};
use glam::{Vec3, Quat, Vec4, Vec2};
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

const CAMERA_SENSITIVITY: f32 = 250.0;
const PLAYER_SPEED: f32 = 100.0;
const JUMP_HEIGHT: f32 = 100.0;
const JUMP_SPEED: f32 = 4.0;
const DASH_DISTANCE: f32 = 100.0;
const FIREFLY_SPEED: f32 = 60.0;

struct Sun {
    noon_pos: Vec3,
    pub light: Light,
    theta: f32
}

impl Sun {
    pub fn new(noon_pos: Vec3, color: Vec3) -> Self {
        let mut sun = Self { noon_pos, light: Light::new(noon_pos, 0.0, color), theta: 0.0 };
        sun.update_theta(sun.theta);
        sun
    }

    pub fn update_theta(&mut self, theta: f32) {
        self.theta = theta % (std::f32::consts::PI * 2.0);
        self.light.position = Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), self.theta) * self.noon_pos;
        self.light.color = Vec3::new(0.7 + 0.1 * self.theta.sin().powf(2.0), 0.2 + 0.8 * self.theta.cos().powf(2.0), 0.8 * self.theta.cos().powf(2.0));
        self.light.strength = self.light.position.length().powf(2.0) * 0.5 * self.light.position.normalize().dot(Vec3::new(0.0, 1.0, 0.0)).powf(1.0/9.0);
        self.light.strength = self.light.strength.max(0.0);
    }

    pub fn frame_finished(&mut self, time: &Time) {
        self.update_theta(self.theta + (time.delta_seconds() * (std::f32::consts::PI / 60.0)));
    }
}

impl Deref for Sun {
    type Target = Light;

    fn deref(&self) -> &Self::Target {
        &self.light
    }
}

struct Player {
    player: RenderObject,
    jump_t: f32
}

impl Player {
    pub fn load(renderer: &mut Renderer, mesh_registry: &mut MeshRegistry, transform: Transform) -> Result<Self, vk::Result> {
       let player = RenderObject::builder(renderer, mesh_registry)
           .set_mesh("player.obj")?
           .set_color(Vec3::new(1.0, 1.0, 1.0))
           .set_transform(transform)
           .build()?;

        Ok(Self { player, jump_t: 0.0 })
    }

    pub fn update_transform<F: Fn(&mut Transform)>(&mut self, predicate: F) {
        predicate(&mut self.player.transform);
    }

    pub fn get_transform(&self) -> Transform {
        self.player.transform.clone()
    }

    pub fn frame_finished(&mut self, keyboard: &Keyboard, mouse: &Mouse, camera: &Camera, time: &Time, viewport: Vec2) {
        // Dash
        if keyboard.is_key_pressed(VirtualKeyCode::Space) && self.jump_t >= (PI / 4.0) { 
            let mouse_direction = (mouse.position - (viewport/2.0)).normalize_or_zero();
            let mouse_direction = camera.get_rotation() * Vec3::new(mouse_direction.x, 0.0, mouse_direction.y);
            self.player.transform.translation += mouse_direction * DASH_DISTANCE;
        }

        // Jump
        if keyboard.is_key_pressed(VirtualKeyCode::Space) && self.jump_t == 0.0 { self.jump_t = std::f32::consts::PI - 0.0001; }

        self.player.transform.translation.y = self.jump_t.sin().powf(0.6) * JUMP_HEIGHT;
        self.jump_t -= time.delta_seconds() * JUMP_SPEED;
        self.jump_t = self.jump_t.max(0.0);

        // Movement
        let z = keyboard.is_key_down(VirtualKeyCode::S) as i32 - keyboard.is_key_down(VirtualKeyCode::W) as i32;
        let x = keyboard.is_key_down(VirtualKeyCode::D) as i32 - keyboard.is_key_down(VirtualKeyCode::A) as i32;
        if x == 0 && z == 0 { return; }
        let delta = Vec3::new(x as f32, 0.0, z as f32).normalize() * PLAYER_SPEED * time.delta_seconds();
        self.player.transform.translation += camera.get_rotation() * delta;  
    }
}

impl Renderable for Player {
    fn get_objects(&self) -> Vec<&RenderObject> {
        vec![&self.player]
    }
}

struct Firefly {
    light: Light,
    velocity: Vec3,
    origin: Vec3,
    firefly: RenderObject
}

impl Firefly {
    pub fn new(renderer: &mut Renderer, mesh_registry: &mut MeshRegistry, position: Vec3, color: Vec3) -> Result<Self, vk::Result> {
        let light = Light::new(position, 0.0, color);

        let firefly = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("firefly.obj")?
            .set_color(Vec3::new(0.0, 0.0, 0.0))
            .set_transform(Transform { translation: position, rotation: Quat::IDENTITY, scale: Vec3::ONE })
            .build()?;
        
        let mut rng = rand::thread_rng();
        let velocity = Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)).normalize_or_zero();
        Ok(Self { firefly, light, velocity, origin: position })
    }

    pub fn frame_finished(&mut self, sun: &Sun, time: &Time) { 
        if sun.theta > (std::f32::consts::PI / 3.0) && sun.theta < (std::f32::consts::PI * (5.0 / 3.0)) {
            self.light.strength = 1000.0 * ((sun.theta / 2.0).sin() - sun.theta.cos()).powf(1.5).min(1.0);
        } else {
            self.light.strength = 0.0
        }

        self.light.position += self.velocity * FIREFLY_SPEED * time.delta_seconds();

        let mut rng = rand::thread_rng();
        let random_vec3 = Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)).normalize_or_zero();
        let origin_direction = (self.origin - self.light.position).normalize_or_zero();
        let origin_bias = ((self.origin - self.light.position).length() - 100.0) / 100.0;
        self.velocity = (self.velocity + random_vec3 * 0.1 + origin_direction * origin_bias).normalize_or_zero(); 

        self.light.position.y = self.light.position.y.clamp(5.0, 15.0);
        self.firefly.transform.translation = self.light.position + Vec3::new(0.0, 5.0, 0.0);
    }
}

impl AsRef<Light> for Firefly {
    fn as_ref(&self) -> &Light {
        &self.light
    }
}

impl Renderable for Firefly {
    fn get_objects(&self) -> Vec<&RenderObject> {
        if (self.light.strength != 0.0) { vec![&self.firefly] }
        else { vec![] }
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
    let mut rng = rand::thread_rng();

    let mut trees: Vec<Tree> = Vec::new();

    for _ in 0..10 {
        let translation = Vec3::new(rng.gen_range(-400.0..400.0), 0.0, rng.gen_range(-400.0..400.0));
        let transform = Transform { translation, rotation: Quat::IDENTITY, scale: Vec3::new(0.5, 0.5, 0.5) };
        trees.push(Tree::load(&mut renderer, &mut mesh_registry, transform).unwrap()); 
    }
    
    let grass = Grass::load(&mut renderer, &mut mesh_registry, Transform::IDENTITY).unwrap();


    let mut sun = Sun::new(Vec3::new(0.0, 1000000.0, 0.0), Vec3::new(0.8, 1.0, 0.5));

    let mut fireflies = Vec::new();

    for _ in 0..10 {
        let position = Vec3::new(rng.gen_range(-400.0..400.0), 10.0, rng.gen_range(-400.0..400.0));
        fireflies.push(Firefly::new(&mut renderer, &mut mesh_registry, position, Vec3::new(0.2, 1.0, 0.4)).unwrap());
    }
   
    let mut player = { 
        let transform = Transform { translation: Vec3::new(0.0, 10.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::new(0.1, 0.1, 0.1) };
        Player::load(&mut renderer, &mut mesh_registry, transform).unwrap()
    };

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

                if keyboard.is_key_pressed(VirtualKeyCode::Up) { sun.light.strength *= 2.0; println!("Multiplier: {}", sun.light.strength / sun.light.position.length()); }
                if keyboard.is_key_pressed(VirtualKeyCode::Down) { sun.light.strength /= 2.0; println!("Multiplier: {}", sun.light.strength / sun.light.position.length()); }

                let mut lights = fireflies.iter().map(|firefly| *firefly.as_ref()).collect::<Vec<Light>>();
                lights.push(*sun);
                renderer.render(&[&grass, &trees, &player, &fireflies], &lights, &camera, &time, &mesh_registry);

                let viewport = Vec2::new(window.inner_size().width as f32, window.inner_size().height as f32);
                player.frame_finished(&keyboard, &mouse, &camera, &time, viewport);
                fireflies.iter_mut().for_each(|firefly| firefly.frame_finished(&sun, &time));
                time.frame_finished();
                keyboard.frame_finished();
                camera.frame_finished();
                mouse.frame_finished();
                sun.frame_finished(&time);
                camera.target = player.get_transform().translation;
            }
            _ => ()
        };

        if let ControlFlow::ExitWithCode(_) = *control_flow {
            println!("Waiting for GPU to finish");
            unsafe { renderer.device.device_wait_idle().unwrap() };
        }
    });
}
