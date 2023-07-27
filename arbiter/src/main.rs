#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use anyhow::Result;
use glam::Vec3;
use net::{ClientboundOpcode, ClientboundPacket, ServerboundOpcode, ServerboundPacket};
use num_traits::FromPrimitive;
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

impl Client {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = bytemuck::cast::<Vec3, [u8; 12]>(self.player_translation).to_vec();
        data.extend(self.username.as_bytes());
        data
    }
}

struct Server {
    socket: UdpSocket,
    connections: HashMap<SocketAddr, Client>,
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
                let packet_size =
                    if let Some(array) = buf.get(0..8).and_then(|bytes| bytes.try_into().ok()) {
                        u64::from_be_bytes(array)
                    } else {
                        warn!("Failed to read packet due to underflow");
                        continue;
                    };

                #[allow(clippy::cast_possible_truncation)]
                let Some(packet) = buf.get(8..(packet_size as usize + 8)) else {
                    warn!("Failed to read packet due to underflow");
                    continue
                };

                let opcode =
                    if let Some(array) = packet.get(0..4).and_then(|bytes| bytes.try_into().ok()) {
                        u32::from_be_bytes(array)
                    } else {
                        warn!("Packet of size {} is too short", packet.len());
                        continue;
                    };

                let Some(opcode) = ServerboundOpcode::from_u32(opcode) else {
                    warn!("Invalid opcode: {}", opcode);
                    continue
                };

                let Some(payload) = packet.get(4..).map(<[u8]>::to_vec) else {
                   warn!("Failed to read packet body");
                   continue
                };

                let packet = ServerboundPacket { opcode, payload };

                handle_packet(&mut server, &packet, addr);
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

fn handle_login(server: &mut Server, packet: &ServerboundPacket, addr: SocketAddr) {
    let username = match String::from_utf8(packet.payload.clone()) {
        Ok(str) => str.trim().to_owned(),
        Err(e) => {
            warn!("Failed to parse username: {}", e);
            if let Err(e) = disconnect(server, addr, Some("Invalid username".to_owned())) {
                warn!("Failed to disconnect client due to {}", e);
            }
            return;
        }
    };

    let client = Client {
        username,
        player_translation: Vec3::new(0.0, 0.0, 0.0),
        last_heartbeat: Instant::now(),
    };

    // Notify peers about new client
    for peer_addr in server.connections.keys() {
        let packet = ClientboundPacket {
            opcode: ClientboundOpcode::SpawnPlayer,
            payload: client.to_bytes(),
        };
        if let Err(e) = server.socket.send_to(&packet.to_bytes(), peer_addr) {
            warn!("Failed to notify {} of new player due to {}", peer_addr, e);
        }
    }

    // Notify client about existing peers
    for (peer_addr, peer_client) in &server.connections {
        let packet = ClientboundPacket {
            opcode: ClientboundOpcode::SpawnPlayer,
            payload: peer_client.to_bytes(),
        };

        if let Err(e) = server.socket.send_to(&packet.to_bytes(), addr) {
            warn!(
                "Failed to notify new player {} of player {} due to {}",
                addr, peer_addr, e
            );
        }
    }

    info!("Added {} to connection list", client.username);
    server.connections.insert(addr, client);
}

fn handle_move(server: &mut Server, packet: &ServerboundPacket, addr: SocketAddr) {
    let Some(client) = server
        .connections
        .get_mut(&addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    let Some(bytes) = packet.payload.get(0..12).and_then(|bytes| bytes.try_into().ok()) else {
        warn!("Failed to parse position from {} moving", client.username);
        return;
    };

    client.player_translation = bytemuck::cast::<[u8; 12], Vec3>(bytes);

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

        let packet = ClientboundPacket {
            opcode: ClientboundOpcode::Move,
            payload: client.to_bytes(),
        };

        if let Err(e) = server.socket.send_to(&packet.to_bytes(), peer_addr) {
            warn!(
                "Failed to notify {} of {} moving due to {}",
                peer_addr, client.username, e
            );
            continue;
        }
    }
}

fn handle_heartbeat(server: &mut Server, _: &ServerboundPacket, addr: SocketAddr) {
    let Some(client) = server
        .connections
        .get_mut(&addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    client.last_heartbeat = Instant::now();
    info!("{} heartbeat", client.username);
}

fn handle_packet(server: &mut Server, packet: &ServerboundPacket, addr: SocketAddr) {
    if matches!(packet.opcode, ServerboundOpcode::Login) {
        handle_login(server, packet, addr);
    }

    if matches!(packet.opcode, ServerboundOpcode::Move) {
        handle_move(server, packet, addr);
    }

    if matches!(packet.opcode, ServerboundOpcode::Heartbeat) {
        handle_heartbeat(server, packet, addr);
    }

    if matches!(packet.opcode, ServerboundOpcode::Disconnect) {
        if let Err(e) = disconnect(server, addr, None) {
            warn!("Failed to disconnect {} due to {}", addr, e);
        }
    }
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

        let packet = ClientboundPacket {
            opcode: ClientboundOpcode::DespawnPlayer,
            payload: client.username.as_bytes().to_vec(),
        };
        server.socket.send_to(&packet.to_bytes(), peer_addr)?;
    }

    if let Some(reason) = reason {
        let packet = ClientboundPacket {
            opcode: ClientboundOpcode::DespawnPlayer,
            payload: reason.as_bytes().to_vec(),
        };
        server.socket.send_to(&packet.payload, addr)?;
    }

    server.connections.remove(&addr);
    Ok(())
}
