use common::net;
use std::{
    net::UdpSocket,
    ops::{Deref, DerefMut},
};

#[derive(thiserror::Error, Debug)]
pub enum PacketSendError {
    #[error("Error sending packet")]
    IOError(#[from] std::io::Error),
    #[error("Error encoding packet")]
    PostcardError(#[from] postcard::Error),
}

pub struct Socket {
    inner: UdpSocket,
}

impl Socket {
    pub fn send(&self, packet: &net::server::Packet) -> Result<(), PacketSendError> {
        let bytes = postcard::to_stdvec(packet)?;
        self.inner.send(&bytes)?;
        Ok(())
    }
}

impl Deref for Socket {
    type Target = UdpSocket;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Socket {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl From<UdpSocket> for Socket {
    fn from(value: UdpSocket) -> Self {
        Self { inner: value }
    }
}
