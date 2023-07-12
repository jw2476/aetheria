#![feature(let_chains)]
#![feature(trivial_bounds)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

extern crate core;

mod camera;
mod input;
mod macros;
mod material;
mod renderer;
mod time;
mod transform;

use anyhow::Result;
use ash::vk;
use assets::{MeshRegistry, ShaderRegistry};
use bytemuck::cast_slice;
use camera::Camera;
use glam::{Quat, Vec2, Vec3, Vec4};
use input::{Keyboard, Mouse};
use net::*;
use num_traits::{FromPrimitive, ToPrimitive};
use rand::Rng;
use renderer::{Light, RenderObject, Renderable, Renderer};
use std::{
    collections::HashMap,
    f32::consts::PI,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    ops::Deref,
    sync::Arc,
    time::{Instant, SystemTime},
};
use time::Time;
use tracing::info;
use transform::Transform;
use vulkan::Context;
use winit::{
    event::{MouseButton, VirtualKeyCode},
    event_loop::ControlFlow,
};

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
    pub fn load(
        renderer: &mut Renderer,
        mesh_registry: &mut MeshRegistry,
        transform: Transform,
    ) -> Result<Tree, vk::Result> {
        let trunk = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("tree.trunk.obj")?
            .set_color(Vec3::new(0.451, 0.243, 0.224))
            .set_transform(transform.clone())
            .build()?;
        let foliage = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("tree.foliage.obj")?
            .set_color(Vec3::new(0.388, 0.780, 0.302))
            .set_transform(transform.clone())
            .build()?;

        Ok(Self {
            transform,
            trunk,
            foliage,
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
    rock: RenderObject,
}

impl Rock {
    pub fn load(
        renderer: &mut Renderer,
        mesh_registry: &mut MeshRegistry,
        transform: Transform,
    ) -> Result<Self, vk::Result> {
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
    grass: RenderObject,
}

impl Grass {
    pub fn load(
        renderer: &mut Renderer,
        mesh_registry: &mut MeshRegistry,
        transform: Transform,
    ) -> Result<Self, vk::Result> {
        let grass = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("grass.obj")?
            .set_color(Vec3::new(0.388, 0.780, 0.302))
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
    theta: f32,
}

impl Sun {
    pub fn new(noon_pos: Vec3, color: Vec3) -> Self {
        let seconds = SystemTime::UNIX_EPOCH.elapsed().unwrap().as_secs();
        let mut sun = Self {
            noon_pos,
            light: Light::new(noon_pos, 0.0, color),
            theta: (seconds % 120) as f32 * (PI / 60.0),
        };
        sun.update_theta(sun.theta);
        sun
    }

    pub fn update_theta(&mut self, theta: f32) {
        self.theta = theta % (std::f32::consts::PI * 2.0);
        self.light.position =
            Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), self.theta) * self.noon_pos;
        self.light.color = Vec3::new(
            0.7 + 0.1 * self.theta.sin().powf(2.0),
            0.2 + 0.8 * self.theta.cos().powf(2.0),
            0.8 * self.theta.cos().powf(2.0),
        );
        self.light.strength = self.light.position.length().powf(2.0)
            * 0.5
            * self.theta.cos().powf(0.13).max(0.0);
        self.light.strength = self.light.strength.max(0.0);
    }

    pub fn frame_finished(&mut self, time: &Time) {
        self.update_theta(self.theta + (time.delta_seconds() * (PI / 60.0)));
    }
}

impl Deref for Sun {
    type Target = Light;

    fn deref(&self) -> &Self::Target {
        &self.light
    }
}

#[derive(Clone)]
struct Player {
    player: RenderObject,
    jump_t: f32,
    light: Light
}

impl Player {
    pub fn load(
        renderer: &mut Renderer,
        mesh_registry: &mut MeshRegistry,
        transform: Transform,
    ) -> Result<Self, vk::Result> {
        let player = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("player.obj")?
            .set_color(Vec3::new(1.0, 1.0, 1.0))
            .set_transform(transform)
            .build()?;

        Ok(Self {
            player,
            jump_t: 0.0,
            light: Light::new(Vec3::ZERO, 5000.0, Vec3::new(0.729, 0.902, 0.992))
        })
    }

    pub fn update_transform<F: Fn(&mut Transform)>(&mut self, predicate: F) {
        predicate(&mut self.player.transform);
    }

    pub fn get_transform(&self) -> Transform {
        self.player.transform.clone()
    }

    pub fn frame_finished(
        &mut self,
        keyboard: &Keyboard,
        mouse: &Mouse,
        camera: &Camera,
        time: &Time,
        viewport: Vec2,
        socket: &UdpSocket,
    ) {
        let old_translation = self.player.transform.translation.clone();

        // Dash
        if keyboard.is_key_pressed(VirtualKeyCode::Space) && self.jump_t >= (PI / 4.0) {
            let mouse_direction = (mouse.position - (viewport / 2.0)).normalize_or_zero();
            let mouse_direction =
                camera.get_rotation() * Vec3::new(mouse_direction.x, 0.0, mouse_direction.y);
            self.player.transform.translation += mouse_direction * DASH_DISTANCE;
        }

        // Jump
        if keyboard.is_key_pressed(VirtualKeyCode::Space) && self.jump_t == 0.0 {
            self.jump_t = std::f32::consts::PI - 0.0001;
        }

        self.player.transform.translation.y = self.jump_t.sin().powf(0.6) * JUMP_HEIGHT;
        self.jump_t -= time.delta_seconds() * JUMP_SPEED;
        self.jump_t = self.jump_t.max(0.0);

        // Movement
        let z = keyboard.is_key_down(VirtualKeyCode::S) as i32
            - keyboard.is_key_down(VirtualKeyCode::W) as i32;
        let x = keyboard.is_key_down(VirtualKeyCode::D) as i32
            - keyboard.is_key_down(VirtualKeyCode::A) as i32;
        if x != 0 || z != 0 {
            let delta = Vec3::new(x as f32, 0.0, z as f32).normalize()
                * PLAYER_SPEED
                * time.delta_seconds();
            self.player.transform.translation += camera.get_rotation() * delta;
        }

        self.light.position = self.player.transform.translation + Vec3::new(0.0, 15.0, 0.0);

        if old_translation != self.player.transform.translation {
            let packet = ServerboundPacket {
                opcode: ServerboundOpcode::Move,
                payload: bytemuck::cast::<Vec3, [u8; 12]>(self.player.transform.translation)
                    .to_vec(),
            };
            socket.send(&packet.to_bytes()).unwrap();
        }
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
    front: RenderObject,
    back: RenderObject,
}

impl Firefly {
    pub fn new(
        renderer: &mut Renderer,
        mesh_registry: &mut MeshRegistry,
        position: Vec3,
        color: Vec3,
    ) -> Result<Self, vk::Result> {
        let light = Light::new(position, 0.0, color);

        let front = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("firefly_front.obj")?
            .set_color(Vec3::new(0.0, 0.0, 0.0))
            .set_transform(Transform {
                translation: position,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            })
            .build()?;
        let back = RenderObject::builder(renderer, mesh_registry)
            .set_mesh("firefly_back.obj")?
            .set_color(Vec3::new(10.0, 10.0, 0.0))
            .set_transform(Transform {
                translation: position,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            })
            .build()?;

        let mut rng = rand::thread_rng();
        let velocity = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        )
        .normalize_or_zero();
        Ok(Self {
            light,
            velocity,
            origin: position,
            front,
            back,
        })
    }

    pub fn frame_finished(&mut self, sun: &Sun, time: &Time) {
        if sun.theta > (std::f32::consts::PI / 3.0)
            && sun.theta < (std::f32::consts::PI * (5.0 / 3.0))
        {
            self.light.strength = 300.0
                * ((sun.theta / 2.0).sin() - sun.theta.cos())
                    .powf(1.5)
                    .min(1.0);
        } else {
            self.light.strength = 0.0
        }

        self.light.position += self.velocity * FIREFLY_SPEED * time.delta_seconds();

        let mut rng = rand::thread_rng();
        let random_vec3 = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        )
        .normalize_or_zero();
        let origin_direction = (self.origin - self.light.position).normalize_or_zero();
        let origin_bias = ((self.origin - self.light.position).length() - 100.0) / 100.0;
        self.velocity = (self.velocity + random_vec3 * 0.1 + origin_direction * origin_bias)
            .normalize_or_zero();

        self.light.position.y = self.light.position.y.clamp(5.0, 15.0);
        self.front.transform.translation = self.light.position + Vec3::new(0.0, 5.0, 0.0);
        self.back.transform.translation = self.light.position + Vec3::new(0.0, 5.0, 0.0);

        let v = Vec3::new(self.velocity.x, 0.0, self.velocity.z).normalize();
        let rotation = Quat::from_rotation_arc(Vec3::new(0.0, 0.0, 1.0), v);
        self.front.transform.rotation = rotation.clone();
        self.back.transform.rotation = rotation.clone();
    }
}

impl AsRef<Light> for Firefly {
    fn as_ref(&self) -> &Light {
        &self.light
    }
}

impl Renderable for Firefly {
    fn get_objects(&self) -> Vec<&RenderObject> {
        if self.light.strength != 0.0 {
            vec![&self.front, &self.back]
        } else {
            vec![]
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let mut ip = String::new();
    println!("Enter server IP: ");
    std::io::stdin().read_line(&mut ip).unwrap();

    if ip.trim().is_empty() {
        ip = "127.0.0.1".to_owned();
    }

    let remote = SocketAddr::new(IpAddr::V4(ip.trim().parse().unwrap()), 8000);
    let socket = UdpSocket::bind("[::]:0").unwrap();
    socket.connect(remote).unwrap();
    socket.set_nonblocking(true).unwrap();
    let mut username = String::new();
    println!("Enter your username: ");
    std::io::stdin().read_line(&mut username).unwrap();
    let login = ServerboundPacket {
        opcode: ServerboundOpcode::Login,
        payload: username.as_bytes().to_vec(),
    };
    socket.send(&login.to_bytes()).unwrap();

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
        let translation = Vec3::new(
            rng.gen_range(-400.0..400.0),
            0.0,
            rng.gen_range(-400.0..400.0),
        );
        let rotation = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), rng.gen_range(-PI..PI));
        let transform = Transform {
            translation,
            rotation,
            scale: Vec3::new(0.1, 0.1, 0.1),
        };
        trees.push(Tree::load(&mut renderer, &mut mesh_registry, transform).unwrap());
    }

    let grass = Grass::load(&mut renderer, &mut mesh_registry, Transform::IDENTITY).unwrap();

    let mut sun = Sun::new(Vec3::new(0.0, 1000000.0, 0.0), Vec3::new(0.8, 1.0, 0.5));

    let mut fireflies = Vec::new();

    for _ in 0..10 {
        let position = Vec3::new(
            rng.gen_range(-400.0..400.0),
            50.0,
            rng.gen_range(-400.0..400.0),
        );
        fireflies.push(
            Firefly::new(
                &mut renderer,
                &mut mesh_registry,
                position,
                Vec3::new(0.745, 0.949, 0.392),
            )
            .unwrap(),
        );
    }

    let mut player = {
        let transform = Transform {
            translation: Vec3::new(0.0, 10.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        Player::load(&mut renderer, &mut mesh_registry, transform).unwrap()
    };

    let mut players: HashMap<String, Player> = HashMap::new();
    let mut last_heartbeat: Instant = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        if let ControlFlow::ExitWithCode(_) = *control_flow {
            return;
        }

        control_flow.set_poll();

        keyboard.on_event(&event);
        mouse.on_event(&event);

        let mut data = [0; 4096];
        match socket.recv(&mut data) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("Waiting for fd");
            }
            Err(e) => panic!("{e}"),
            Ok(n) => {
                let packet_size = u64::from_be_bytes(data[0..8].try_into().unwrap());
                println!("Packet size: {}", n);
                let packet = &data[8..(packet_size as usize + 8)];
                let packet = ClientboundPacket {
                    opcode: ClientboundOpcode::from_u32(u32::from_be_bytes(
                        packet[0..4].try_into().unwrap(),
                    ))
                    .unwrap(),
                    payload: packet[4..].to_vec(),
                };

                if let ClientboundOpcode::SpawnPlayer = packet.opcode {
                    info!("Spawning player");
                    let translation =
                        bytemuck::cast::<[u8; 12], Vec3>(packet.payload[0..12].try_into().unwrap());
                    let username = String::from_utf8(packet.payload[12..].to_vec()).unwrap();
                    players.insert(
                        username,
                        Player::load(
                            &mut renderer,
                            &mut mesh_registry,
                            Transform {
                                translation,
                                rotation: Quat::IDENTITY,
                                scale: Vec3::ONE,
                            },
                        )
                        .unwrap(),
                    );
                }

                if let ClientboundOpcode::Move = packet.opcode {
                    info!("Moving peer player");
                    let translation =
                        bytemuck::cast::<[u8; 12], Vec3>(packet.payload[0..12].try_into().unwrap());
                    let username = String::from_utf8(packet.payload[12..].to_vec()).unwrap();
                    players
                        .get_mut(&username)
                        .expect("Peer not found")
                        .player
                        .transform
                        .translation = translation;
                }

                if let ClientboundOpcode::DespawnPlayer = packet.opcode {
                    info!("Deleting peer player");
                    let username = String::from_utf8(packet.payload).unwrap();
                    players.remove(&username);
                }

                if let ClientboundOpcode::NotifyDisconnection = packet.opcode {
                    info!("Disconnecting due to server request");
                    control_flow.set_exit();
                    return;
                }
            }
        };

        if last_heartbeat.elapsed().as_secs_f32() > 10.0 {
            heartbeat(&socket).unwrap();
            last_heartbeat = Instant::now();
        }

        match event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::Resized(size) => {
                    renderer.recreate_swapchain().unwrap();
                    camera.width = size.width as f32;
                    camera.height = size.height as f32;
                }
                winit::event::WindowEvent::CloseRequested => {
                    disconnect(&socket).unwrap();
                    control_flow.set_exit()
                }
                _ => (),
            },
            winit::event::Event::MainEventsCleared => {
                if keyboard.is_key_down(VirtualKeyCode::Escape) {
                    disconnect(&socket).unwrap();
                    control_flow.set_exit()
                }
                if mouse.is_button_down(MouseButton::Right) {
                    camera.theta -= mouse.delta.x / CAMERA_SENSITIVITY
                }
                if keyboard.is_key_down(VirtualKeyCode::Left) {
                    sun.theta += PI / 60.0
                }
                if keyboard.is_key_down(VirtualKeyCode::Right) {
                    sun.theta -= PI / 60.0
                }

                let mut lights = fireflies
                    .iter()
                    .map(|firefly| *firefly.as_ref())
                    .collect::<Vec<Light>>();
                lights.push(*sun);
                lights.push(player.light);
                renderer.render(
                    &[
                        &grass,
                        &trees,
                        &player,
                        &fireflies,
                        &players.values().cloned().collect::<Vec<Player>>(),
                    ],
                    &lights,
                    &camera,
                    &time,
                    &mesh_registry,
                );
                let viewport = Vec2::new(
                    window.inner_size().width as f32,
                    window.inner_size().height as f32,
                );
                player.frame_finished(&keyboard, &mouse, &camera, &time, viewport, &socket);
                fireflies
                    .iter_mut()
                    .for_each(|firefly| firefly.frame_finished(&sun, &time));
                time.frame_finished();
                keyboard.frame_finished();
                camera.frame_finished();
                mouse.frame_finished();
                sun.frame_finished(&time);
                camera.target = player.get_transform().translation;
            }
            _ => (),
        };

        if let ControlFlow::ExitWithCode(_) = *control_flow {
            println!("Waiting for GPU to finish");
            unsafe { renderer.device.device_wait_idle().unwrap() };
        }
    });
}

fn heartbeat(socket: &UdpSocket) -> Result<()> {
    let packet = ServerboundPacket {
        opcode: ServerboundOpcode::Heartbeat,
        payload: Vec::new(),
    };
    socket.send(&packet.to_bytes())?;
    Ok(())
}

fn disconnect(socket: &UdpSocket) -> Result<()> {
    let packet = ServerboundPacket {
        opcode: ServerboundOpcode::Disconnect,
        payload: Vec::new(),
    };
    socket.send(&packet.to_bytes())?;
    Ok(())
}
