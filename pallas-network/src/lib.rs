//! Implementation of the Ouroboros networking stack.
//!
//! A multiplexer plus state-machines for each Cardano mini-protocol
//! (handshake, chain-sync, block-fetch, tx-submission, local-state-query,
//! …), exposed through ergonomic per-role facades. Async, tokio-backed.
//!
//! This is the original, single-connection / client-server-shaped stack. A
//! peer-to-peer rewrite is in progress in `pallas-network2`; once that is
//! mature it is intended to replace this crate.
//!
//! # Usage
//!
//! ```no_run
//! use pallas_network::facades::PeerClient;
//! use pallas_network::miniprotocols::MAINNET_MAGIC;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let mut peer = PeerClient::connect(
//!     "relays-new.cardano-mainnet.iohk.io:3001",
//!     MAINNET_MAGIC,
//! ).await?;
//!
//! let _chainsync  = peer.chainsync();   // &mut chainsync::N2NClient
//! let _blockfetch = peer.blockfetch();  // &mut blockfetch::Client
//! # Ok(()) }
//! ```
//!
//! # Overview
//!
//! - [`facades`] — opinionated client / server bundles per role:
//!   [`facades::PeerClient`] / [`facades::PeerServer`] (N2N),
//!   [`facades::NodeClient`] / [`facades::NodeServer`] (N2C),
//!   [`facades::DmqClient`] / [`facades::DmqServer`]. Each owns a
//!   multiplexer and exposes the relevant mini-protocols through accessor
//!   methods.
//! - [`multiplexer`] — the segment-level transport:
//!   [`multiplexer::RunningPlexer`], [`multiplexer::Bearer`].
//! - [`miniprotocols`] — every Ouroboros mini-protocol: `handshake`,
//!   `chainsync`, `blockfetch`, `txsubmission`, `localstate`,
//!   `localtxsubmission`, `txmonitor`, `keepalive`, `peersharing`,
//!   `localmsgnotification`, `localmsgsubmission`. Network-magic constants
//!   ([`miniprotocols::MAINNET_MAGIC`], [`miniprotocols::TESTNET_MAGIC`],
//!   [`miniprotocols::PREVIEW_MAGIC`], [`miniprotocols::PREPROD_MAGIC`],
//!   [`miniprotocols::SANCHONET_MAGIC`]) are re-exported from this module.
//!
//! # Usage as part of `pallas`
//!
//! When depending on the umbrella [`pallas`] crate, this crate is re-exported
//! as `pallas::network`.
//!
//! [`pallas`]: https://crates.io/crates/pallas

/// High-level client/server facades (node-to-node, node-to-client).
pub mod facades;
/// State-machines for every Ouroboros mini-protocol (handshake, chain-sync,
/// block-fetch, tx-submission, local-state-query, …) and the shared
/// network-magic constants.
pub mod miniprotocols;
/// Segment-level transport that multiplexes mini-protocol traffic over a
/// single bearer.
pub mod multiplexer;
