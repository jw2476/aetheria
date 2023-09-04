#![feature(let_chains)]
#![feature(trivial_bounds)]
#![feature(associated_type_defaults)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

extern crate core;

mod camera;
mod components;
mod data;
mod entities;
mod input;
mod macros;
mod renderer;
mod scenes;
mod socket;
mod systems;
mod time;
mod ui;

use anyhow::Result;
use ash::vk;
use assets::{ModelRegistry, ShaderRegistry, TextureRegistry, Transform};
use bytemuck::cast_slice;
use camera::Camera;
use common::{
    item::{Item, ItemStack},
    net, Observable, Observer,
};
use glam::{IVec2, Quat, UVec2, Vec2, Vec3, Vec4};
use input::{Keyboard, Mouse};
use num_traits::FromPrimitive;
use std::{
    collections::HashMap,
    f32::consts::PI,
    io,
    net::{IpAddr, SocketAddr, UdpSocket},
    ops::DerefMut,
    sync::{Arc, Mutex},
    time::Instant,
};
use time::Time;
use tracing::info;
use vulkan::Context;
use winit::{
    event::{MouseButton, VirtualKeyCode},
    event_loop::ControlFlow,
};

use crate::{
    components::{craft, recipe_selector},
    data::{inventory::Inventory, Data},
    entities::{Player, Tree},
    renderer::{Renderer, RENDER_HEIGHT, RENDER_WIDTH},
    scenes::RootScene,
    socket::Socket,
    systems::{interact, render, Systems},
    ui::{Element, Rectangle, Region, SizeConstraints, UIPass},
};

use dialog::DialogBox;

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

    let mut ip = dialog::Input::new("Enter Server IP:")
        .title("IP")
        .show()
        .expect("Failed to show IP dialog box")
        .unwrap_or("".to_owned());

    if ip.trim().is_empty() {
        ip = "127.0.0.1".to_owned();
    }

    let remote = SocketAddr::new(IpAddr::V4(ip.trim().parse().unwrap()), 8000);
    let socket: Arc<Socket> = Arc::new(UdpSocket::bind("[::]:0").unwrap().into());
    socket.connect(remote).unwrap();
    socket.set_nonblocking(true).unwrap();

    let username = dialog::Input::new("Enter username:")
        .title("Username")
        .show()
        .expect("Failed to show username dialog box");

    if username.is_none() || username.as_ref().unwrap().trim().is_empty() {
        dialog::Message::new("Username cannot be empty")
            .title("Username error")
            .show()
            .expect("Failed to show error dialog box");

        return;
    }
    let username = username.unwrap();

    let login = net::server::Packet::Login(net::server::Login {
        username: username.trim().to_owned(),
    });

    socket.send(&login).unwrap();

    let (event_loop, window) = create_window();
    let window = Arc::new(window);
    let ctx = Context::new(&window);

    let mut model_registry = ModelRegistry::new();
    let mut shader_registry = ShaderRegistry::new();
    let mut texture_registry = TextureRegistry::new();

    let mut renderer = Renderer::new(ctx, window.clone()).unwrap();
    let mut camera = Camera::new(480.0, 270.0, &renderer).unwrap();
    let mut time = Time::new(&renderer).unwrap();
    let render_system = Arc::new(Mutex::new(
        render::System::new(&renderer, &mut shader_registry, &camera, &time).unwrap(),
    ));
    let interact_system = Arc::new(Mutex::new(interact::System::new()));

    let mut data = Data {
        inventory: Inventory::new(socket.clone()),
        current_recipe: None,
        recipe_selections: None,
    };

    let ui_pass = Arc::new(Mutex::new(
        UIPass::new(
            &mut renderer,
            &mut shader_registry,
            &mut texture_registry,
            render_system.lock().unwrap().get_texture(),
        )
        .unwrap(),
    ));
    renderer.add_pass(render_system.clone());
    renderer.add_pass(ui_pass.clone());
    renderer.set_output_image(
        ui_pass.lock().unwrap().get_texture().image.clone(),
        vk::ImageLayout::GENERAL,
    );
    let mut keyboard = Keyboard::new();
    let mut mouse = Mouse::new();

    let mut root = RootScene::new(
        &mut renderer,
        &mut Systems {
            render: &mut render_system.lock().unwrap(),
            interact: &mut interact_system.lock().unwrap(),
        },
        &mut model_registry,
    )
    .expect("Failed to load scene");

    interact_system
        .lock()
        .unwrap()
        .set_player(root.player.clone());

    let mut players: HashMap<String, Arc<Mutex<Player>>> = HashMap::new();
    let mut last_heartbeat: Instant = Instant::now();

    let mut inventory_open = false;

    event_loop.run(move |event, _, control_flow| {
        if let ControlFlow::ExitWithCode(_) = *control_flow {
            return;
        }

        control_flow.set_poll();

        keyboard.on_event(&event);
        mouse.on_event(&event);

        let mut buf = [0; 4096];
        match socket.recv(&mut buf) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
            Ok(_) => {
                let packet: net::client::Packet = postcard::from_bytes(&buf).unwrap();

                match packet {
                    net::client::Packet::SpawnPlayer(packet) => {
                        info!("Spawning player");
                        players.insert(
                            packet.username,
                            Player::new(
                                &mut renderer,
                                &mut Systems {
                                    render: &mut render_system.lock().unwrap(),
                                    interact: &mut interact_system.lock().unwrap(),
                                },
                                &mut model_registry,
                                Transform {
                                    translation: packet.position,
                                    rotation: Quat::IDENTITY,
                                    scale: Vec3::ONE,
                                },
                            )
                            .unwrap(),
                        );
                    }
                    net::client::Packet::Move(packet) => {
                        info!("Moving peer player");
                        players
                            .get_mut(&packet.username)
                            .expect("Peer not found")
                            .lock()
                            .unwrap()
                            .player
                            .transform
                            .translation = packet.position;
                    }
                    net::client::Packet::DespawnPlayer(packet) => {
                        info!("Deleting peer player");
                        players.remove(&packet.username);
                    }
                    net::client::Packet::NotifyDisconnection(packet) => {
                        info!("Disconnecting due to {}", packet.reason);
                        control_flow.set_exit();
                        return;
                    }
                    net::client::Packet::ModifyInventory(packet) => {
                        info!("Setting {:?} to {}", packet.stack.item, packet.stack.amount);
                        data.inventory.set(packet.stack);
                    }
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
                    let mut sun = root.sun.lock().unwrap();
                    let theta = sun.get_theta() + PI / 60.0;
                    sun.update_theta(theta);
                }
                if keyboard.is_key_down(VirtualKeyCode::Right) {
                    let mut sun = root.sun.lock().unwrap();
                    let theta = sun.get_theta() - PI / 60.0;
                    sun.update_theta(theta);
                }

                if keyboard.is_key_pressed(VirtualKeyCode::I) {
                    inventory_open = !inventory_open;
                }

                renderer.wait_for_frame();
                render_system
                    .lock()
                    .unwrap()
                    .set_geometry(&data, &renderer, &model_registry);

                let mut scene = Vec::new();

                interact_system
                    .lock()
                    .unwrap()
                    .frame_finished(&camera, &keyboard, &mut scene, &mut data);

                if let Some(mut component) = craft::Component::new(&mut data, &mouse) {
                    let size = component.layout(SizeConstraints {
                        min: UVec2::new(0, 0),
                        max: UVec2::new(480, 270),
                    });
                    component.paint(
                        Region {
                            origin: UVec2::new(0, 0),
                            size,
                        },
                        &mut scene,
                    )
                }

                if let Some(mut component) = recipe_selector::Component::new(&mut data, &mouse) {
                    let size = component.layout(SizeConstraints {
                        min: UVec2::new(0, 0),
                        max: UVec2::new(480, 270),
                    });
                    component.paint(
                        Region {
                            origin: UVec2::new(0, 0),
                            size,
                        },
                        &mut scene,
                    )
                }

                if inventory_open {
                    let mut inventory_window =
                        components::inventory::Component::new(&data.inventory);
                    let size = inventory_window.layout(SizeConstraints {
                        min: UVec2::new(0, 0),
                        max: UVec2::new(RENDER_WIDTH, RENDER_HEIGHT),
                    });
                    inventory_window.paint(
                        Region {
                            origin: UVec2::new(480 - (size.x + 2), 270 - (size.y + 2)),
                            size,
                        },
                        &mut scene,
                    );
                }
                ui_pass
                    .lock()
                    .unwrap()
                    .set_geometry(&renderer, &scene)
                    .expect("Failed to set UI geometry");

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
                camera.target = root.player.lock().unwrap().player.transform.translation;

                println!("{}", mouse.position);
            }
            _ => (),
        };

        if let ControlFlow::ExitWithCode(_) = *control_flow {
            println!("Waiting for GPU to finish");
            unsafe { renderer.device.device_wait_idle().unwrap() };
        }
    });
}

fn heartbeat(socket: &Socket) -> Result<()> {
    let packet = net::server::Packet::Heartbeat;
    socket.send(&packet)?;
    Ok(())
}

fn disconnect(socket: &Socket) -> Result<()> {
    let packet = net::server::Packet::Disconnect;
    socket.send(&packet)?;
    Ok(())
}
