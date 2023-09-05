#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use anyhow::Result;
use async_std::net::UdpSocket;
use common::{
    item::{Item, ItemStack},
    net,
};
use glam::Vec3;
use num_traits::{FromPrimitive, ToPrimitive};
use sqlx::SqlitePool;
use std::{
    collections::{
        hash_map::{Keys, Values},
        HashMap,
    },
    hash::Hash,
    net::SocketAddr,
    ops::Deref,
    time::Instant,
};
use tracing::{error, info, warn};

#[derive(Clone, PartialEq, Eq)]
struct Connection {
    last_heartbeat: Instant,
    addr: SocketAddr,
    user_id: i64,
    character_id: i64,
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

impl Deref for Connection {
    type Target = SocketAddr;

    fn deref(&self) -> &Self::Target {
        &self.addr
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
    online: IndexedMap<Connection>,
    pool: SqlitePool,
}

#[derive(thiserror::Error, Debug)]
enum SendError {
    #[error("IO error")]
    IOError(#[from] std::io::Error),
    #[error("Encode error")]
    EncodeError(#[from] postcard::Error),
}

impl Server {
    pub fn new(socket: UdpSocket, pool: SqlitePool) -> Self {
        Self {
            socket,
            online: IndexedMap::new(),
            pool,
        }
    }

    pub async fn send(
        &self,
        addr: &SocketAddr,
        packet: &net::client::Packet,
    ) -> Result<(), SendError> {
        let bytes = postcard::to_stdvec(packet)?;
        self.socket.send_to(&bytes, addr).await?;
        Ok(())
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let socket = UdpSocket::bind("0.0.0.0:8000").await?;

    let pool = SqlitePool::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&mut pool.acquire().await?).await?;

    let mut server = Server::new(socket, pool);
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
    let Ok(user) = sqlx::query!(
        "SELECT id, password FROM users WHERE username = ?",
        packet.username
    )
    .fetch_optional(&server.pool)
    .await
    else {
        error!("Fetching user {} failed", packet.username);
        return;
    };

    let Some(user) = user else {
        let _ = send_error(server, addr, "No account found for that username", true).await;
        return;
    };

    if user.password != packet.password {
        let _ = send_error(server, addr, "Username or password is incorrect", true).await;
        return;
    }

    let Ok(character) = sqlx::query!(
        "SELECT id, name, position_x, position_y, position_z FROM characters WHERE owner = ?",
        user.id
    )
    .fetch_one(&server.pool)
    .await
    else {
        error!("Fetching character for user {} failed", packet.username);
        return;
    };
    let position = Vec3::new(
        character.position_x as f32,
        character.position_y as f32,
        character.position_z as f32,
    );

    server.online.insert(Connection {
        last_heartbeat: Instant::now(),
        addr,
        user_id: user.id,
        character_id: character.id,
    });

    let connection = server
        .online
        .get(&addr)
        .expect("Failed to get connection that was just inserted, this is very bad");

    for peer in server.online.values() {
        // Notify peers about new client
        let packet = net::client::Packet::SpawnPlayer(net::client::SpawnPlayer {
            username: packet.username.clone(),
            position,
        });

        if let Err(e) = server.send(peer, &packet).await {
            warn!("Failed to notify {} of new player due to {}", peer.addr, e);
        }

        let Ok(peer_user) = sqlx::query!("SELECT username FROM users WHERE id = ?", peer.user_id)
            .fetch_one(&server.pool)
            .await
        else {
            error!("Fetching peer user {} failed", peer.user_id);
            continue;
        };

        let Ok(peer_character) = sqlx::query!(
            "SELECT position_x, position_y, position_z FROM characters WHERE id = ?",
            peer.character_id
        )
        .fetch_one(&server.pool)
        .await
        else {
            error!("Fetching peer character {} failed", peer.character_id);
            continue;
        };

        let peer_position = Vec3::new(
            peer_character.position_x as f32,
            peer_character.position_y as f32,
            peer_character.position_z as f32,
        );

        // Notify new client about peers
        let packet = net::client::Packet::SpawnPlayer(net::client::SpawnPlayer {
            username: peer_user.username.clone(),
            position: peer_position,
        });

        if let Err(e) = server.send(connection, &packet).await {
            warn!(
                "Failed to notify new player {} of player {} due to {}",
                addr, peer.addr, e
            );
        }
    }

    let Ok(items) = sqlx::query!(
        "SELECT item, quantity FROM items WHERE owner = ?",
        character.id
    )
    .fetch_all(&server.pool)
    .await
    else {
        error!(
            "Fetching items for character {} user {} failed",
            character.name, packet.username
        );
        return;
    };

    // Set clients inventory
    for stack in items {
        let Some(item) = Item::from_i64(stack.item) else {
            error!("Invalid item ID in database {}", stack.item);
            continue;
        };

        let inventory_packet = net::client::Packet::ModifyInventory(net::client::ModifyInventory {
            stack: ItemStack {
                item,
                amount: stack.quantity as u32,
            },
        });

        if let Err(e) = server.send(connection, &inventory_packet).await {
            warn!(
                "Failed to update player {}'s inventory stack {:?} due to {}",
                packet.username, stack, e
            );
            continue;
        }

        info!("Updating player {}'s stack {:?}", packet.username, stack);
    }

    info!("Added {} to connection list", packet.username);
}

async fn handle_move(server: &mut Server, packet: &net::server::Move, addr: SocketAddr) {
    let Some(connection) = server.online.get_mut(&addr) else {
        warn!("Cannot find client for addr {}", addr);
        return;
    };

    if let Err(e) = sqlx::query!(
        "UPDATE characters SET position_x = ?, position_y = ?, position_z = ? WHERE id = ?",
        packet.position.x,
        packet.position.y,
        packet.position.z,
        connection.character_id
    )
    .execute(&server.pool)
    .await
    {
        error!(
            "Updating position for character {} failed due to {}",
            connection.character_id, e
        );
        return;
    }

    info!(
        "Updated position for {} to {:?}",
        connection.character_id, packet.position
    );

    let Some(connection) = server.online.get(&addr) else {
        warn!("Cannot find client for addr {}", addr);
        return;
    };

    let Ok(user) = sqlx::query!(
        "SELECT username FROM users WHERE id = ?",
        connection.user_id
    )
    .fetch_one(&server.pool)
    .await
    else {
        error!("Failed to fetch user with id {}", connection.user_id);
        return;
    };

    for peer in server.online.values().filter(|peer| peer != &connection) {
        let packet = net::client::Packet::Move(net::client::Move {
            username: user.username.clone(),
            position: packet.position,
        });

        if let Err(e) = server.send(peer, &packet).await {
            warn!(
                "Failed to notify {} of {} moving due to {}",
                peer.addr, user.username, e
            );
            continue;
        }
    }
}

fn handle_heartbeat(server: &mut Server, addr: SocketAddr) {
    let Some(connection) = server.online.get_mut(&addr) else {
        warn!("Cannot find client for addr {}", addr);
        return;
    };

    connection.last_heartbeat = Instant::now();
    info!("{} heartbeat", connection.user_id);
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
            handle_modify_inventory(server, packet, addr).await
        }
        net::server::Packet::Signup(packet) => handle_signup(server, packet, addr).await,
    };

    Ok(())
}

async fn disconnect(server: &mut Server, addr: SocketAddr, reason: Option<String>) -> Result<()> {
    let Some(connection) = server.online.get(&addr) else {
        warn!("Cannot find client for addr {}", addr);
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Client not found").into());
    };
    info!("{} is disconnecting", connection.user_id);

    let user = sqlx::query!(
        "SELECT username FROM users WHERE id = ?",
        connection.user_id
    )
    .fetch_one(&server.pool)
    .await?;

    for peer in server.online.values().filter(|peer| peer != &connection) {
        let packet = net::client::Packet::DespawnPlayer(net::client::DespawnPlayer {
            username: user.username.clone(),
        });

        server.send(peer, &packet).await?;
    }

    if let Some(reason) = reason {
        let packet =
            net::client::Packet::NotifyDisconnection(net::client::NotifyDisconnection { reason });
        server.send(&connection, &packet).await?;
    }

    server.online.remove(&connection.get_unique_key());

    Ok(())
}

async fn send_error(server: &Server, addr: SocketAddr, message: &str, fatal: bool) {
    let packet = net::client::Packet::DisplayError(net::client::DisplayError {
        message: message.to_owned(),
        fatal,
    });
    let _ = server.send(&addr, &packet).await;
}

async fn handle_modify_inventory(
    server: &mut Server,
    packet: &net::server::ModifyInventory,
    addr: SocketAddr,
) {
    let Some(connection) = server.online.get_mut(&addr) else {
        warn!("Cannot find client for addr {}", addr);
        return;
    };

    let item = packet.stack.item.to_i64();

    let Ok(existing) = sqlx::query!(
        "SELECT id FROM items WHERE owner = ? AND item = ?",
        connection.character_id,
        item
    )
    .fetch_optional(&server.pool)
    .await
    else {
        error!(
            "Failed to fetch existing item stack character {} item {}",
            connection.character_id, packet.stack.item
        );
        return;
    };

    let result = if let Some(_) = existing {
        sqlx::query!("UPDATE items SET quantity = ?", packet.stack.amount)
            .execute(&server.pool)
            .await
    } else {
        sqlx::query!(
            "INSERT INTO items (item, quantity, owner) VALUES (?, ?, ?)",
            item,
            packet.stack.amount,
            connection.character_id
        )
        .execute(&server.pool)
        .await
    };

    if let Err(e) = result {
        error!(
            "Failed to set item stack {:?} for character {} due to {}",
            packet.stack, connection.character_id, e
        );
    }
}

async fn handle_signup(server: &Server, packet: &net::server::Signup, addr: SocketAddr) {
    let Ok(existing) = sqlx::query!("SELECT id FROM users WHERE username = ?", packet.username)
        .fetch_optional(&server.pool)
        .await
    else {
        error!("Failed to fetch existing user for signup");
        send_error(server, addr, "Server error", true).await;
        return;
    };

    if let Some(_) = existing {
        send_error(server, addr, "User exists with that username", true).await;
        return;
    }

    if let Err(e) = sqlx::query!(
        "INSERT INTO users (username, password) VALUES (?, ?)",
        packet.username,
        packet.password
    )
    .execute(&server.pool)
    .await
    {
        error!("Failed to create new user due to {}", e);
        send_error(server, addr, "Server error", true).await;
        return;
    }

    let Ok(user) = sqlx::query!("SELECT id FROM users WHERE username = ?", packet.username)
        .fetch_one(&server.pool)
        .await
    else {
        error!("Newly created user cannot be found");
        send_error(server, addr, "Server error", true).await;
        return;
    };

    if let Err(e) = sqlx::query!("INSERT INTO characters (name, position_x, position_y, position_z, owner) VALUES (?, ?, ?, ?, ?)", packet.username, 0.0, 0.0, 0.0, user.id).execute(&server.pool).await {
        error!("Failed to insert new character for {} due to {}", packet.username, e);
        send_error(server, addr, "Server error", true).await;
        return;
    }
}
