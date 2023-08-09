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
    addr: Option<SocketAddr>
}

struct Server {
    socket: UdpSocket,
    clients: Vec<Client>,
}

#[derive(thiserror::Error, Debug)]
enum SendError {
    #[error("IO error")]
    IOError(#[from] std::io::Error),
    #[error("Encode error")]
    EncodeError(#[from] postcard::Error),
}

impl Server {
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket,
            clients: Vec::new()
        }
    }

    pub fn send(&self, addr: SocketAddr, packet: &net::client::Packet) -> Result<(), SendError> {
        let bytes = postcard::to_stdvec(packet)?;
        self.socket.send_to(&bytes, addr)?;
        Ok(())
    }

    pub fn get_client(&self, addr: SocketAddr) -> Option<&Client> {
        self.clients.iter().filter(|client| client.addr.is_some()).find(|client| client.addr.unwrap() == addr)
    }

    pub fn get_client_mut(&mut self, addr: SocketAddr) -> Option<&mut Client> {
        self.clients.iter_mut().filter(|client| client.addr.is_some()).find(|client| client.addr.unwrap() == addr)
    }

    pub fn active_clients(&self) -> impl Iterator<Item=&Client> {
        self.clients.iter().filter(|client| client.addr.is_some())
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let socket = UdpSocket::bind("0.0.0.0:8000")?;
    socket.set_nonblocking(true)?;
    let mut server = Server {
        socket,
        clients: Vec::new(),
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
    for client in server.clients.clone() {
        if client.addr.is_none() { continue; }

        if client.last_heartbeat.elapsed().as_secs_f32() > 20.0 {
            if let Err(e) = disconnect(server, client.addr.unwrap(), Some("Heartbeat timeout".to_owned())) {
                warn!("Failed to disconnect {} due to {}", client.username, e);
                continue;
            }

            server.get_client_mut(client.addr.unwrap()).unwrap().addr = None
        }
    }

    Ok(())
}

fn handle_login(server: &mut Server, packet: &net::server::Login, addr: SocketAddr) {
    let client = if let Some(client) = server
        .clients
        .iter()
        .find(|client| client.username == packet.username)
    {
        client.clone()
    } else {
        let client = Client {
            username: packet.username.clone(),
            player_translation: Vec3::new(0.0, 0.0, 0.0),
            last_heartbeat: Instant::now(),
            inventory: Inventory::new(),
            addr: None
        };
        server.clients.push(client);
        server.clients.last().unwrap().clone()
    };
    server.clients.iter_mut().find(|client| client.username == packet.username).unwrap().addr = Some(addr);

    // Notify peers about new client
    for peer in server.active_clients().filter(|client| client.username != packet.username) {
        let packet = net::client::Packet::SpawnPlayer(net::client::SpawnPlayer {
            username: client.username.clone(),
            position: client.player_translation,
        });
        if let Err(e) = server.send(peer.addr.unwrap(), &packet) {
            warn!("Failed to notify {} of new player due to {}", client.addr.unwrap(), e);
        }
    }

    // Notify client about existing peers
    for peer in server.clients.iter().filter(|client| client.addr.is_some()) {
        if peer.username == packet.username {
            continue;
        }

        let packet = net::client::Packet::SpawnPlayer(net::client::SpawnPlayer {
            username: peer.username.clone(),
            position: peer.player_translation,
        });

        if let Err(e) = server.send(addr, &packet) {
            warn!(
                "Failed to notify new player {} of player {} due to {}",
                addr, peer.addr.unwrap(), e
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

    for peer in server.active_clients() {
        if peer.addr.unwrap() == addr {
            continue;
        }

        let packet = net::client::Packet::Move(net::client::Move {
            username: client.username.clone(),
            position: client.player_translation,
        });

        if let Err(e) = server.send(peer.addr.unwrap(), &packet) {
            warn!(
                "Failed to notify {} of {} moving due to {}",
                peer.addr.unwrap(), client.username, e
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

    for peer in server.active_clients() {
        if peer.addr.unwrap() == addr {
            continue;
        }

        let packet = net::client::Packet::DespawnPlayer(net::client::DespawnPlayer {
            username: client.username.clone(),
        });

        server.send(peer.addr.unwrap(), &packet)?;
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
