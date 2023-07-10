use anyhow::Result;
use glam::Vec3;
use net::*;
use num_traits::{FromPrimitive, ToPrimitive};
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    sync::Arc,
    time::Instant,
};
use tracing::info;

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
                let packet_size = u64::from_be_bytes(buf[0..8].try_into().unwrap());
                let packet = &buf[8..(packet_size as usize + 8)];

                let packet = ServerboundPacket {
                    opcode: ServerboundOpcode::from_u32(u32::from_be_bytes(
                        packet[0..4].try_into().unwrap(),
                    ))
                    .expect("Invalid opcode"),
                    payload: packet[4..].to_vec(),
                };

                handle_packet(&mut server, packet, addr);
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

fn handle_packet(server: &mut Server, packet: ServerboundPacket, addr: SocketAddr) {
    if let ServerboundOpcode::Login = packet.opcode {
        let client = Client {
            username: String::from_utf8(packet.payload.clone())
                .unwrap()
                .trim()
                .to_owned(),
            player_translation: Vec3::new(0.0, 0.0, 0.0),
            last_heartbeat: Instant::now(),
        };

        // Notify peers about new client
        for peer_addr in server.connections.keys() {
            let packet = ClientboundPacket {
                opcode: ClientboundOpcode::SpawnPlayer,
                payload: client.to_bytes(),
            };
            server
                .socket
                .send_to(&packet.to_bytes(), peer_addr)
                .unwrap();
        }

        // Notify client about existing peers
        for peer_client in server.connections.values() {
            let packet = ClientboundPacket {
                opcode: ClientboundOpcode::SpawnPlayer,
                payload: peer_client.to_bytes(),
            };
            server.socket.send_to(&packet.to_bytes(), addr).unwrap();
        }

        info!("Added {} to connection list", client.username);
        server.connections.insert(addr, client);
    }

    if let ServerboundOpcode::Move = packet.opcode {
        let client = server
            .connections
            .get_mut(&addr)
            .expect("No client found with address {addr}");
        client.player_translation =
            bytemuck::cast::<[u8; 12], Vec3>(packet.payload.clone()[0..12].try_into().unwrap());
        info!(
            "Updated position for {} to {:?}",
            client.username, client.player_translation
        );
        let client = server
            .connections
            .get(&addr)
            .expect("No client found with address {addr}");

        for peer_addr in server.connections.keys() {
            if *peer_addr == addr {
                continue;
            }

            let packet = ClientboundPacket {
                opcode: ClientboundOpcode::Move,
                payload: client.to_bytes(),
            };
            server
                .socket
                .send_to(&packet.to_bytes(), peer_addr)
                .unwrap();
        }
    }

    if let ServerboundOpcode::Heartbeat = packet.opcode {
        let client = server
            .connections
            .get_mut(&addr)
            .expect("No client found with address {addr}");
        client.last_heartbeat = Instant::now();
        info!("{} heartbeat", client.username);
    }

    if let ServerboundOpcode::Disconnect = packet.opcode {
        disconnect(server, addr, None).unwrap();
    }
}

fn disconnect(server: &mut Server, addr: SocketAddr, reason: Option<String>) -> Result<()> {
    let client = server
        .connections
        .get(&addr)
        .expect("No client found with address {addr}");
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
