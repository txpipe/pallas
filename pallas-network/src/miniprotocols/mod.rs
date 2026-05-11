//! Implementations for the different Ouroboros mini-protocols

mod common;

/// Block-fetch mini-protocol: download block bodies for a given chain range.
pub mod blockfetch;
/// Chain-sync mini-protocol: follow a peer's chain and stream new blocks/headers.
pub mod chainsync;
/// Handshake mini-protocol: negotiate the protocol version on a fresh connection.
pub mod handshake;
/// Keep-alive mini-protocol: liveness pings to detect dead peers.
pub mod keepalive;
/// Local-message-notification mini-protocol (DMQ): receive notifications from the node.
pub mod localmsgnotification;
/// Local-message-submission mini-protocol (DMQ): submit messages to the node.
pub mod localmsgsubmission;
/// Local-state-query mini-protocol: query the node's ledger state at a given point.
pub mod localstate;
/// Local-tx-submission mini-protocol: submit transactions to a local node.
pub mod localtxsubmission;
/// Peer-sharing mini-protocol: discover other peers known to the connected node.
pub mod peersharing;
/// Tx-monitor mini-protocol: observe the local node's mempool.
pub mod txmonitor;
/// Tx-submission mini-protocol: gossip transactions between nodes.
pub mod txsubmission;

pub use common::*;
