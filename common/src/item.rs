use serde::{Deserialize, Serialize};
use std::{fmt::Display, ops::Deref};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Wood,
    Fireglow,
    Lamp,
}

impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Wood => "Wood",
                Self::Fireglow => "Fireglow",
                Self::Lamp => "Lamp",
            }
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemStack {
    pub item: Item,
    pub amount: u32,
}

impl Display for ItemStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} x{}", self.item, self.amount)
    }
}
