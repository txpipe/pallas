//! Implementations for the different Ouroboros mini-protocols

mod common;

pub mod blockfetch;
pub mod chainsync;
pub mod handshake;
pub mod keepalive;
pub mod peersharing;
pub mod txsubmission;

pub use common::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn other(msg: impl std::fmt::Display) -> Self {
        Self::Other(msg.to_string())
    }
}
