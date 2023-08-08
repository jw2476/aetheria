#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use anyhow::Result;
use common::{item::Inventory, net};
use glam::Vec3;
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    time::Instant,
};
use tracing::{error, info, warn};

#[derive(Clone)]
struct Client {
    player_translation: Vec3,
    username: String,
    last_heartbeat: Instant,
    inventory: Inventory,
}

struct Server {
    socket: UdpSocket,
    clients: Vec<Client>,
    connections: HashMap<SocketAddr, usize>,
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

    pub fn get_client(&self, addr: SocketAddr) -> Option<&Client> {
        self.connections
            .get(&addr)
            .and_then(|&idx| self.clients.get(idx))
    }

    pub fn get_client_mut(&mut self, addr: SocketAddr) -> Option<&mut Client> {
        self.connections
            .get(&addr)
            .and_then(|&idx| self.clients.get_mut(idx))
    }

    pub fn get_connections(&self) -> impl Iterator<Item = (SocketAddr, &Client)> {
        self.connections
            .iter()
            .map(|(&addr, idx)| (addr, &self.clients[*idx]))
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let socket = UdpSocket::bind("0.0.0.0:8000")?;
    socket.set_nonblocking(true)?;
    let mut server = Server {
        socket,
        clients: Vec::new(),
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
    for (addr, id) in server.connections.clone() {
        let client = server.clients[id].clone();

        if client.last_heartbeat.elapsed().as_secs_f32() > 20.0 {
            if let Err(e) = disconnect(server, addr, Some("Heartbeat timeout".to_owned())) {
                warn!("Failed to disconnect {} due to {}", client.username, e);
                continue;
            }

            server.connections.remove(&addr);
        }
    }

    Ok(())
}

fn handle_login(server: &mut Server, packet: &net::server::Login, addr: SocketAddr) {
    let (client_id, client) = if let Some(client) = server
        .clients
        .iter()
        .enumerate()
        .find(|(_, client)| client.username == packet.username)
    {
        client
    } else {
        let client = Client {
            username: packet.username.clone(),
            player_translation: Vec3::new(0.0, 0.0, 0.0),
            last_heartbeat: Instant::now(),
            inventory: Inventory::new(),
        };
        server.clients.push(client);
        (server.clients.len() - 1, server.clients.last().unwrap())
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
    for (peer_addr, peer_client) in server.get_connections() {
        if peer_client.username == packet.username {
            continue;
        }

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

    // Set clients inventory
    for stack in client.inventory.get_items() {
        let packet =
            net::client::Packet::ModifyInventory(net::client::ModifyInventory { stack: *stack });

        if let Err(e) = server.send(addr, &packet) {
            warn!(
                "Failed to update player {}'s inventory stack {:?} due to {}",
                client.username, stack, e
            );
            continue;
        }

        info!("Updating player {}'s stack {:?}", client.username, stack);
    }

    info!("Added {} to connection list", client.username);
    server.connections.insert(addr, client_id);

    server.get_client_mut(addr).unwrap().last_heartbeat = Instant::now();
}

fn handle_move(server: &mut Server, packet: &net::server::Move, addr: SocketAddr) {
    let Some(client) = server
        .get_client_mut(addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    client.player_translation = packet.position;

    info!(
        "Updated position for {} to {:?}",
        client.username, client.player_translation
    );

    let Some(client) = server
        .get_client(addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    for peer_addr in server.connections.keys() {
        if *peer_addr == addr {
            continue;
        }

        let packet = net::client::Packet::Move(net::client::Move {
            username: client.username.clone(),
            position: client.player_translation,
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
        .get_client_mut(addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    client.last_heartbeat = Instant::now();
    info!("{} heartbeat", client.username);
}

fn handle_packet(
    server: &mut Server,
    packet: &net::server::Packet,
    addr: SocketAddr,
) -> Result<()> {
    match packet {
        net::server::Packet::Login(packet) => handle_login(server, packet, addr),
        net::server::Packet::Move(packet) => handle_move(server, packet, addr),
        net::server::Packet::Heartbeat => handle_heartbeat(server, addr),
        net::server::Packet::Disconnect => disconnect(server, addr, None)?,
        net::server::Packet::ModifyInventory(packet) => {
            handle_modify_inventory(server, packet, addr)
        }
    };

    Ok(())
}

fn disconnect(server: &Server, addr: SocketAddr, reason: Option<String>) -> Result<()> {
    let Some(client) = server
        .get_client(addr) else {
            warn!("Cannot find client for addr {}", addr);
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Client not found").into());
        };
    info!("{} is disconnecting", client.username);

    for peer_addr in server.connections.keys() {
        if *peer_addr == addr {
            continue;
        }

        let packet = net::client::Packet::DespawnPlayer(net::client::DespawnPlayer {
            username: client.username.clone(),
        });

        server.send(*peer_addr, &packet)?;
    }

    if let Some(reason) = reason {
        let packet =
            net::client::Packet::NotifyDisconnection(net::client::NotifyDisconnection { reason });
        server.send(addr, &packet)?;
    }

    Ok(())
}

fn handle_modify_inventory(
    server: &mut Server,
    packet: &net::server::ModifyInventory,
    addr: SocketAddr,
) {
    let Some(client) = server
        .get_client_mut(addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    client.inventory.set(packet.stack);

    println!("{:?}", packet.stack);
}
