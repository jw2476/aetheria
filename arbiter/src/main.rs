#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use anyhow::Result;
use common::{item::Inventory, net};
use glam::Vec3;
use std::{
    collections::{
        hash_map::{Keys, Values},
        HashMap, HashSet,
    },
    hash::{Hash, Hasher},
    net::{SocketAddr, UdpSocket},
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
    time::Instant,
};
use tracing::{error, info, warn};

#[derive(Clone)]
struct Player {
    position: Vec3,
    username: String,
    inventory: Inventory,
}

#[derive(Clone)]
struct Connection {
    last_heartbeat: Instant,
    addr: SocketAddr,
    player: Player,
}

impl Deref for Connection {
    type Target = Player;

    fn deref(&self) -> &Self::Target {
        &self.player
    }
}

impl DerefMut for Connection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.player
    }
}

trait Unique {
    type Key: Eq + PartialEq + Hash;

    fn get_unique_key(&self) -> Self::Key;
}

impl Unique for Connection {
    type Key = SocketAddr;

    fn get_unique_key(&self) -> Self::Key {
        self.addr
    }
}

impl Unique for Player {
    type Key = String;

    fn get_unique_key(&self) -> Self::Key {
        self.username.clone()
    }
}

struct IndexedMap<T>
where
    T: Unique,
{
    inner: HashMap<T::Key, T>,
}

impl<T> IndexedMap<T>
where
    T: Unique + Clone,
{
    fn new() -> Self {
        Self::default()
    }

    fn get(&self, key: &T::Key) -> Option<&T> {
        self.inner.get(key)
    }

    fn get_mut(&mut self, key: &T::Key) -> Option<&mut T> {
        self.inner.get_mut(key)
    }

    fn insert(&mut self, value: T) {
        self.inner.insert(value.get_unique_key(), value);
    }

    fn remove(&mut self, key: &T::Key) {
        self.inner.remove(key);
    }

    fn values<'a>(&'a self) -> Values<'a, T::Key, T> {
        self.inner.values()
    }

    fn keys<'a>(&'a self) -> Keys<'a, T::Key, T> {
        self.inner.keys()
    }

    fn take(&mut self, key: &T::Key) -> Option<T> {
        let value = self.get(key).cloned();
        self.remove(key);
        value
    }
}

impl<T> Default for IndexedMap<T>
where
    T: Unique,
{
    fn default() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
}

struct Server {
    socket: UdpSocket,
    offline: IndexedMap<Player>,
    online: IndexedMap<Connection>,
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
            offline: IndexedMap::new(),
            online: IndexedMap::new(),
        }
    }

    pub fn send(
        &self,
        connection: &Connection,
        packet: &net::client::Packet,
    ) -> Result<(), SendError> {
        let bytes = postcard::to_stdvec(packet)?;
        self.socket.send_to(&bytes, connection.addr)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let socket = UdpSocket::bind("0.0.0.0:8000")?;
    socket.set_nonblocking(true)?;
    let mut server = Server::new(socket);
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
    let dead = server
        .online
        .values()
        .filter(|connection| connection.last_heartbeat.elapsed().as_secs_f32() > 20.0)
        .map(|connection| connection.addr)
        .collect::<Vec<SocketAddr>>();

    for addr in dead {
        if let Err(e) = disconnect(server, addr, Some("Heartbeat timeout".to_owned())) {
            warn!("Failed to disconnect {} due to {}", addr, e);
            continue;
        }
    }

    Ok(())
}

fn handle_login(server: &mut Server, packet: &net::server::Login, addr: SocketAddr) {
    let player = server.offline.take(&packet.username).unwrap_or(Player {
        position: Vec3::ZERO,
        username: packet.username.clone(),
        inventory: Inventory::new()
    });
    
    server.online.insert(Connection { last_heartbeat: Instant::now(), addr, player });
    let connection = server.online.get(&addr).expect("Failed to get connection that was just inserted, this is very bad");

    for peer in server
        .online
        .values()
        .filter(|peer| peer.player.username != packet.username)
    {
        // Notify peers about new client
        let packet = net::client::Packet::SpawnPlayer(net::client::SpawnPlayer {
            username: connection.player.username.clone(),
            position: connection.player.position,
        });

        if let Err(e) = server.send(peer, &packet) {
            warn!("Failed to notify {} of new player due to {}", peer.addr, e);
        }

        // Notify new client about peers
        let packet = net::client::Packet::SpawnPlayer(net::client::SpawnPlayer {
            username: peer.player.username.clone(),
            position: peer.player.position,
        });

        if let Err(e) = server.send(connection, &packet) {
            warn!(
                "Failed to notify new player {} of player {} due to {}",
                addr, peer.addr, e
            );
        }
    }

    // Set clients inventory
    for stack in connection.player.inventory.get_items() {
        let packet =
            net::client::Packet::ModifyInventory(net::client::ModifyInventory { stack: *stack });

        if let Err(e) = server.send(connection, &packet) {
            warn!(
                "Failed to update player {}'s inventory stack {:?} due to {}",
                connection.player.username, stack, e
            );
            continue;
        }

        info!(
            "Updating player {}'s stack {:?}",
            connection.player.username, stack
        );
    }

    info!("Added {} to connection list", connection.player.username);
}

fn handle_move(server: &mut Server, packet: &net::server::Move, addr: SocketAddr) {
    let Some(connection) = server.online
        .get_mut(&addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    connection.player.position = packet.position;

    info!(
        "Updated position for {} to {:?}",
        connection.player.username, connection.player.position
    );

    let Some(connection) = server.online
        .get(&addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    for peer in server
        .online
        .values()
        .filter(|peer| peer.username != connection.username)
    {
        let packet = net::client::Packet::Move(net::client::Move {
            username: connection.player.username.clone(),
            position: connection.player.position,
        });

        if let Err(e) = server.send(peer, &packet) {
            warn!(
                "Failed to notify {} of {} moving due to {}",
                peer.addr, connection.player.username, e
            );
            continue;
        }
    }
}

fn handle_heartbeat(server: &mut Server, addr: SocketAddr) {
    let Some(connection) = server.online
        .get_mut(&addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    connection.last_heartbeat = Instant::now();
    info!("{} heartbeat", connection.username);
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

fn disconnect(server: &mut Server, addr: SocketAddr, reason: Option<String>) -> Result<()> {
    let Some(connection) = server.online
        .get(&addr) else {
            warn!("Cannot find client for addr {}", addr);
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Client not found").into());
        };
    info!("{} is disconnecting", connection.player.username);

    for peer in server
        .online
        .values()
        .filter(|peer| peer.username != connection.username)
    {
        let packet = net::client::Packet::DespawnPlayer(net::client::DespawnPlayer {
            username: connection.player.username.clone(),
        });

        server.send(peer, &packet)?;
    }

    if let Some(reason) = reason {
        let packet =
            net::client::Packet::NotifyDisconnection(net::client::NotifyDisconnection { reason });
        server.send(&connection, &packet)?;
    }

    server.offline.insert(connection.player.clone());
    server.online.remove(&connection.get_unique_key());

    Ok(())
}

fn handle_modify_inventory(
    server: &mut Server,
    packet: &net::server::ModifyInventory,
    addr: SocketAddr,
) {
    let Some(connection) = server.online
        .get_mut(&addr) else {
            warn!("Cannot find client for addr {}", addr);
            return;
        };

    connection.player.inventory.set(packet.stack);

    println!("{:?}", packet.stack);
}
