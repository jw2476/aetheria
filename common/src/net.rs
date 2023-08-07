mod common {
    use serde::{Deserialize, Serialize};
    use crate::item::ItemStack;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ModifyInventory {
        pub stack: ItemStack
    }
}

pub mod server {
    use serde::{Deserialize, Serialize};
    pub use super::common::ModifyInventory;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Login {
        pub username: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Move {
        pub position: glam::Vec3,
    }


    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Packet {
        Login(Login),
        Move(Move),
        Heartbeat,
        Disconnect,
        ModifyInventory(ModifyInventory)
    }
}

pub mod client {
    use serde::{Deserialize, Serialize};
    pub use super::common::ModifyInventory;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct SpawnPlayer {
        pub username: String,
        pub position: glam::Vec3,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct DespawnPlayer {
        pub username: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Move {
        pub username: String,
        pub position: glam::Vec3,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct NotifyDisconnection {
        pub reason: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Packet {
        SpawnPlayer(SpawnPlayer),
        DespawnPlayer(DespawnPlayer),
        Move(Move),
        NotifyDisconnection(NotifyDisconnection),
        ModifyInventory(ModifyInventory)
    }
}
