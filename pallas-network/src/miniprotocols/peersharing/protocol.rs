use std::fmt::Debug;

use std::net::{Ipv4Addr, Ipv6Addr};

use crate::miniprotocols::localstate::queries_v16::primitives::Port;

/// Number of peer addresses requested or returned.
pub type Amount = u8;

/// Peer-sharing state-machine state.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Idle: client may request peers or terminate.
    Idle,
    /// Server is gathering the requested number of peers.
    Busy(Amount),
    /// Protocol terminated.
    Done,
}

/// An IPv4 or IPv6 peer endpoint.
#[derive(Debug, PartialEq, Clone)]
pub enum PeerAddress {
    /// IPv4 address paired with a TCP port.
    V4(Ipv4Addr, Port),
    /// IPv6 address paired with a TCP port.
    V6(Ipv6Addr, Port),
}

/// Peer-sharing protocol message.
#[derive(Debug)]
pub enum Message {
    /// Client → server: ask for up to `Amount` peer addresses.
    ShareRequest(Amount),
    /// Server → client: reply with the discovered peer addresses.
    SharePeers(Vec<PeerAddress>),
    /// Client → server: terminate the protocol.
    Done,
}
