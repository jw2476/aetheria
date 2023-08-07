use serde::{Serialize, Deserialize};
use std::ops::Deref;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Wood,
    Fireglow,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemStack {
    pub item: Item,
    pub amount: u32,
}

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
