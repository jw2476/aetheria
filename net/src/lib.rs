use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;

#[derive(FromPrimitive, ToPrimitive)]
pub enum ServerboundOpcode {
    Login,
    Move,
    Heartbeat,
    Disconnect,
}

pub struct ServerboundPacket {
    pub opcode: ServerboundOpcode,
    pub payload: Vec<u8>,
}

#[derive(FromPrimitive, ToPrimitive)]
pub enum ClientboundOpcode {
    SpawnPlayer,
    Move,
    DespawnPlayer,
    NotifyDisconnection,
    Kick,
}

pub struct ClientboundPacket {
    pub opcode: ClientboundOpcode,
    pub payload: Vec<u8>,
}

impl ClientboundPacket {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(&self.opcode.to_u32().unwrap().to_be_bytes());
        data.extend(&self.payload.clone());
        let mut packet = data.len().to_be_bytes().to_vec();
        packet.append(&mut data);
        packet
    }
}

impl ServerboundPacket {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(&self.opcode.to_u32().unwrap().to_be_bytes());
        data.extend(&self.payload.clone());
        let mut packet = data.len().to_be_bytes().to_vec();
        packet.append(&mut data);
        packet
    }
}
