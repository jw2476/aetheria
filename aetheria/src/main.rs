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
mod entities;
mod scenes;

use anyhow::Result;
use ash::vk;
use assets::{MeshRegistry, ShaderRegistry};
use bytemuck::cast_slice;
use camera::Camera;
use glam::{Quat, Vec2, Vec3, Vec4};
use input::{Keyboard, Mouse};
use net::*;
use num_traits::{FromPrimitive};
use rand::Rng;
use renderer::{Light, Renderer};
use std::{
    collections::HashMap,
    f32::consts::PI,
    io,
    net::{IpAddr, SocketAddr, UdpSocket},
    sync::Arc,
    time::Instant,
};
use time::Time;
use tracing::info;
use transform::Transform;
use vulkan::Context;
use winit::{
    event::{MouseButton, VirtualKeyCode},
    event_loop::ControlFlow,
};

use crate::{entities::Player, scenes::RootScene, renderer::{RenderPass, UIPass, Rectangle}};

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

const CAMERA_SENSITIVITY: f32 = 250.0;

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

    let mut renderer = Renderer::new(ctx, window.clone()).unwrap();
    let mut camera = Camera::new(480.0, 270.0, &renderer).unwrap();
    let mut time = Time::new(&renderer).unwrap();
    let render_pass = Arc::new(RenderPass::new(&renderer, &mut shader_registry, &camera, &time).unwrap());
    let ui_pass = Arc::new(UIPass::new(&renderer, &mut shader_registry, render_pass.get_texture()).unwrap());
    renderer.add_pass(render_pass.clone());
    renderer.add_pass(ui_pass.clone());
    renderer.set_output_image(ui_pass.get_texture().image.clone(), vk::ImageLayout::GENERAL);
    let mut keyboard = Keyboard::new();
    let mut mouse = Mouse::new();

    let mut root = RootScene::new(&mut renderer, &mut mesh_registry).expect("Failed to load scene");

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
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
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
                        Player::new(
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
                        .update_transform(|transform| transform.translation = translation);
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
                    root.sun.update_theta(root.sun.get_theta() + PI / 60.0)
                }
                if keyboard.is_key_down(VirtualKeyCode::Right) {
                    root.sun.update_theta(root.sun.get_theta() - PI / 60.0)
                }

                renderer.wait_for_frame();
                render_pass.set_geometry(
                    &renderer, 
                    &mesh_registry,
                    &[
                        &root,
                        &players.values().cloned().collect::<Vec<Player>>(),
                    ], 
                    &root.get_lights(),
                );
                ui_pass.set_geometry(&renderer, &[Rectangle { origin: Vec2::new(50.0, 50.0), extent: Vec2::new(20.0, 20.0), radius: 10.0, color: Vec4::new(1.0, 1.0, 0.0, 0.4), ..Default::default() }]).unwrap();
                renderer.render();
                let viewport = Vec2::new(
                    window.inner_size().width as f32,
                    window.inner_size().height as f32,
                );

                root.frame_finished(&keyboard, &mouse, &camera, &time, viewport, &socket);
                time.frame_finished();
                keyboard.frame_finished();
                camera.frame_finished();
                mouse.frame_finished();
                camera.target = root.player.get_transform().translation;
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
