#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use anyhow::Result;
use async_std::net::UdpSocket;
use common::{item::ItemStack, net};
use glam::Vec3;
use std::{
    collections::{
        hash_map::{Keys, Values},
        HashMap,
    },
    hash::Hash,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    time::Instant,
};
use tracing::{error, info, warn};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    inventory: Vec<ItemStack>,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            inventory: Vec::new(),
        }
    }

    pub fn add(&mut self, stack: ItemStack) {
        if let Some(existing) = self.inventory.iter_mut().find(|s| s.item == stack.item) {
            existing.amount += stack.amount;
        } else {
            self.inventory.push(stack);
        }
    }

    pub fn set(&mut self, stack: ItemStack) {
        if let Some(existing) = self.inventory.iter_mut().find(|s| s.item == stack.item) {
            existing.amount = stack.amount;
        } else {
            self.inventory.push(stack);
        }
    }

    pub fn get_items(&self) -> &[ItemStack] {
        &self.inventory
    }
}

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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &T::Key) -> Option<&T> {
        self.inner.get(key)
    }

    pub fn get_mut(&mut self, key: &T::Key) -> Option<&mut T> {
        self.inner.get_mut(key)
    }

    pub fn insert(&mut self, value: T) {
        self.inner.insert(value.get_unique_key(), value);
    }

    pub fn remove(&mut self, key: &T::Key) {
        self.inner.remove(key);
    }

    pub fn values<'a>(&'a self) -> Values<'a, T::Key, T> {
        self.inner.values()
    }

    pub fn keys<'a>(&'a self) -> Keys<'a, T::Key, T> {
        self.inner.keys()
    }

    pub fn take(&mut self, key: &T::Key) -> Option<T> {
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

    pub async fn send(
        &self,
        connection: &Connection,
        packet: &net::client::Packet,
    ) -> Result<(), SendError> {
        let bytes = postcard::to_stdvec(packet)?;
        self.socket.send_to(&bytes, connection.addr).await?;
        Ok(())
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let socket = UdpSocket::bind("0.0.0.0:8000").await?;
    let mut server = Server::new(socket);
    info!("Listening on 0.0.0.0:8000");

    let mut last_heartbeat_check = Instant::now();

    loop {
        let mut buf = [0; 4096];
        match server.socket.recv_from(&mut buf).await {
            Err(e) => panic!("{e}"),
            Ok((_, addr)) => {
                let packet = match postcard::from_bytes(&buf) {
                    Ok(packet) => packet,
                    Err(e) => {
                        warn!("Failed to decode packet due to {}", e);
                        continue;
                    }
                };

                if let Err(e) = handle_packet(&mut server, &packet, addr).await {
                    warn!("Handling packet failed with {e}");
                    continue;
                }
            }
        }

        if last_heartbeat_check.elapsed().as_secs_f32() > 1.0 {
            info!("Checking heartbeats");
            check_heartbeats(&mut server).await?;
            last_heartbeat_check = Instant::now();
        }
    }
}

async fn check_heartbeats(server: &mut Server) -> Result<()> {
    let dead = server
        .online
        .values()
        .filter(|connection| connection.last_heartbeat.elapsed().as_secs_f32() > 20.0)
        .map(|connection| connection.addr)
        .collect::<Vec<SocketAddr>>();

    for addr in dead {
        if let Err(e) = disconnect(server, addr, Some("Heartbeat timeout".to_owned())).await {
            warn!("Failed to disconnect {} due to {}", addr, e);
            continue;
        }
    }

    Ok(())
}

async fn handle_login(server: &mut Server, packet: &net::server::Login, addr: SocketAddr) {
    let player = server.offline.take(&packet.username).unwrap_or(Player {
        position: Vec3::ZERO,
        username: packet.username.clone(),
        inventory: Inventory::new(),
    });

    server.online.insert(Connection {
        last_heartbeat: Instant::now(),
        addr,
        player,
    });
    let connection = server
        .online
        .get(&addr)
        .expect("Failed to get connection that was just inserted, this is very bad");

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

        if let Err(e) = server.send(peer, &packet).await {
            warn!("Failed to notify {} of new player due to {}", peer.addr, e);
        }

        // Notify new client about peers
        let packet = net::client::Packet::SpawnPlayer(net::client::SpawnPlayer {
            username: peer.player.username.clone(),
            position: peer.player.position,
        });

        if let Err(e) = server.send(connection, &packet).await {
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

        if let Err(e) = server.send(connection, &packet).await {
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

async fn handle_move(server: &mut Server, packet: &net::server::Move, addr: SocketAddr) {
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

        if let Err(e) = server.send(peer, &packet).await {
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

async fn handle_packet(
    server: &mut Server,
    packet: &net::server::Packet,
    addr: SocketAddr,
) -> Result<()> {
    match packet {
        net::server::Packet::Login(packet) => handle_login(server, packet, addr).await,
        net::server::Packet::Move(packet) => handle_move(server, packet, addr).await,
        net::server::Packet::Heartbeat => handle_heartbeat(server, addr),
        net::server::Packet::Disconnect => disconnect(server, addr, None).await?,
        net::server::Packet::ModifyInventory(packet) => {
            handle_modify_inventory(server, packet, addr)
        }
    };

    Ok(())
}

async fn disconnect(server: &mut Server, addr: SocketAddr, reason: Option<String>) -> Result<()> {
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

        server.send(peer, &packet).await?;
    }

    if let Some(reason) = reason {
        let packet =
            net::client::Packet::NotifyDisconnection(net::client::NotifyDisconnection { reason });
        server.send(&connection, &packet).await?;
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
