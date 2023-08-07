#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use anyhow::Result;
use common::net;
use glam::Vec3;
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    time::Instant,
};
use tracing::{info, warn};

#[derive(Clone)]
struct Client {
    player_translation: Vec3,
    username: String,
    last_heartbeat: Instant,
}

struct Server {
    socket: UdpSocket,
    connections: HashMap<SocketAddr, Client>,
}

#[derive(thiserror::Error, Debug)]
enum SendError {
    #[error("IO error")]
    IOError(#[from] std::io::Error),
    #[error("Encode error")]
    EncodeError(#[from] postcard::Error),
}

impl Server {
    pub fn send(&self, addr: SocketAddr, packet: &net::client::Packet) -> Result<(), SendError> {
        let bytes = postcard::to_stdvec(packet)?;
        self.socket.send_to(&bytes, addr)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let socket = UdpSocket::bind("0.0.0.0:8000")?;
    socket.set_nonblocking(true)?;
    let mut server = Server {
        socket,
        connections: HashMap::new(),
    };
    info!("Listening on 0.0.0.0:8000");

    let mut last_heartbeat_check = Instant::now();

    loop {
        let mut buf = [0; 4096];
        match server.socket.recv_from(&mut buf) {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
            Ok((_, addr)) => {
                let packet = match postcard::from_bytes(&buf) {
                    Ok(packet) => packet,
                    Err(e) => {
                        warn!("Failed to decode packet due to {}", e);
                        continue;
                    }
                };

                if let Err(e) = handle_packet(&mut server, &packet, addr) {
                    warn!("Handling packet failed with {e}");
                    continue;
                }
            }
        }

        if last_heartbeat_check.elapsed().as_secs_f32() > 1.0 {
            info!("Checking heartbeats");
            check_heartbeats(&mut server)?;
            last_heartbeat_check = Instant::now();
        }
    }
}

fn check_heartbeats(server: &mut Server) -> Result<()> {
    for (addr, client) in server.connections.clone() {
        if client.last_heartbeat.elapsed().as_secs_f32() > 20.0 {
            disconnect(server, addr, Some("Heartbeat timeout".to_owned()))?;
        }
    }

    Ok(())
}

fn handle_login(server: &mut Server, packet: &net::server::Login, addr: SocketAddr) {
    let client = Client {
        username: packet.username.clone(),
        player_translation: Vec3::new(0.0, 0.0, 0.0),
        last_heartbeat: Instant::now(),
    };

    // Notify peers about new client
    for peer_addr in server.connections.keys() {
        let packet = net::client::Packet::SpawnPlayer(net::client::SpawnPlayer {
            username: client.username.clone(),
            position: client.player_translation,
        });
        if let Err(e) = server.send(*peer_addr, &packet) {
            warn!("Failed to notify {} of new player due to {}", peer_addr, e);
        }
    }

    // Notify client about existing peers
    for (peer_addr, peer_client) in &server.connections {
        let packet = net::client::Packet::SpawnPlayer(net::client::SpawnPlayer {
            username: peer_client.username.clone(),
            position: peer_client.player_translation,
        });
        if let Err(e) = server.send(addr, &packet) {
            warn!(
                "Failed to notify new player {} of player {} due to {}",
                addr, peer_addr, e
            );
        }
    }

    info!("Added {} to connection list", client.username);
    server.connections.insert(addr, client);
}

fn handle_move(server: &mut Server, packet: &net::server::Move, addr: SocketAddr) {
    let Some(client) = server
        .connections
        .get_mut(&addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    client.player_translation = packet.position;

    info!(
        "Updated position for {} to {:?}",
        client.username, client.player_translation
    );

    let Some(client) = server
        .connections
        .get(&addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    for peer_addr in server.connections.keys() {
        if *peer_addr == addr {
            continue;
        }

        let packet = net::client::Packet::Move(net::client::Move {
            username: client.username.clone(),
            position: client.player_translation
        });

        if let Err(e) = server.send(*peer_addr, &packet) {
            warn!(
                "Failed to notify {} of {} moving due to {}",
                peer_addr, client.username, e
            );
            continue;
        }
    }
}

fn handle_heartbeat(server: &mut Server, addr: SocketAddr) {
    let Some(client) = server
        .connections
        .get_mut(&addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    client.last_heartbeat = Instant::now();
    info!("{} heartbeat", client.username);
}

fn handle_packet(server: &mut Server, packet: &net::server::Packet, addr: SocketAddr) -> Result<()> {
    match packet {
        net::server::Packet::Login(packet) => handle_login(server, packet, addr),
        net::server::Packet::Move(packet) => handle_move(server, packet, addr),
        net::server::Packet::Heartbeat => handle_heartbeat(server, addr),
        net::server::Packet::Disconnect => disconnect(server, addr, None)?        
    };

    Ok(())
}

fn disconnect(server: &mut Server, addr: SocketAddr, reason: Option<String>) -> Result<()> {
    let Some(client) = server
        .connections
        .get(&addr) else {
            warn!("Cannot find client for addr {}", addr);
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Client not found").into());
        };
    info!("{} is disconnecting", client.username);

    for peer_addr in server.connections.keys() {
        if *peer_addr == addr {
            continue;
        }

        let packet = net::client::Packet::DespawnPlayer(net::client::DespawnPlayer {
            username: client.username.clone()
        });

        server.send(addr, &packet)?;
    }

    if let Some(reason) = reason {
        let packet = net::client::Packet::NotifyDisconnection(net::client::NotifyDisconnection {
            reason
        });
        server.send(addr, &packet)?;
    }

    server.connections.remove(&addr);
    Ok(())
}
