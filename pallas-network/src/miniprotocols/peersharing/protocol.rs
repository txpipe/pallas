use std::fmt::Debug;

use std::net::{Ipv4Addr, Ipv6Addr};

use crate::miniprotocols::localstate::queries_v16::primitives::Port;

pub type Amount = u8;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    Busy(Amount),
    Done,
}

#[derive(Debug, PartialEq, Clone)]
pub enum PeerAddress {
    V4(Ipv4Addr, Port),
    V6(Ipv6Addr, Port),
}

#[derive(Debug)]
pub enum Message {
    ShareRequest(Amount),
    SharePeers(Vec<PeerAddress>),
    Done,
}
