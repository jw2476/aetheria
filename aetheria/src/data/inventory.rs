use common::{
    item::{Item, ItemStack},
    net,
};
use std::sync::Arc;
use tracing::warn;

use crate::socket::Socket;

#[derive(Clone)]
pub struct Inventory {
    inventory: Vec<ItemStack>,
    socket: Arc<Socket>,
}

impl Inventory {
    pub fn new(socket: Arc<Socket>) -> Self {
        Self {
            inventory: Vec::new(),
            socket,
        }
    }

    fn update(&self, item: Item) {
        let Some(stack) = self.inventory.iter().find(|s| s.item == item) else {
            warn!("Tried to update stack {:?} that doesn't exist", item);
            return;
        };

        let packet = net::server::Packet::ModifyInventory(net::server::ModifyInventory {
            stack: stack.clone(),
        });
        if let Err(e) = self.socket.send(&packet) {
            warn!("Failed to update stack {:?} due to {}", item, e);
            return;
        }
    }

    pub fn add(&mut self, stack: ItemStack) {
        if let Some(existing) = self.inventory.iter_mut().find(|s| s.item == stack.item) {
            existing.amount += stack.amount;
        } else {
            self.inventory.push(stack);
        }

        self.update(stack.item);
    }

    pub fn set(&mut self, stack: ItemStack) {
        if let Some(existing) = self.inventory.iter_mut().find(|s| s.item == stack.item) {
            existing.amount = stack.amount;
        } else {
            self.inventory.push(stack);
        }

        self.update(stack.item);
    }

    pub fn get_items(&self) -> &[ItemStack] {
        &self.inventory
    }
}
