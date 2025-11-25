/// Well-known magic for testnet
#[deprecated(note = "Use `pallas_primitives::types::network_constant::TESTNET_MAGIC` instead")]
pub const TESTNET_MAGIC: u64 = pallas_primitives::types::network_constant::TESTNET_MAGIC;

/// Well-known magic for mainnet
#[deprecated(note = "Use `pallas_primitives::types::network_constant::MAINNET_MAGIC` instead")]
pub const MAINNET_MAGIC: u64 = pallas_primitives::types::network_constant::MAINNET_MAGIC;

/// Well-known magic for preview
#[deprecated(note = "Use `pallas_primitives::types::network_constant::PREVIEW_MAGIC` instead")]
pub const PREVIEW_MAGIC: u64 = pallas_primitives::types::network_constant::PREVIEW_MAGIC;

/// Well-known magic for preprod
#[deprecated(note = "Use `pallas_primitives::types::network_constant::PREPROD_MAGIC` instead")]
pub const PREPROD_MAGIC: u64 = pallas_primitives::types::network_constant::PREPROD_MAGIC;

/// Alias for PREPROD_MAGIC
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PRE_PRODUCTION_MAGIC` instead"
)]
pub const PRE_PRODUCTION_MAGIC: u64 =
    pallas_primitives::types::network_constant::PRE_PRODUCTION_MAGIC;

/// Well-known magic for preprod
#[deprecated(note = "Use `pallas_primitives::types::network_constant::SANCHONET_MAGIC` instead")]
pub const SANCHONET_MAGIC: u64 = pallas_primitives::types::network_constant::SANCHONET_MAGIC;

/// Bitflag for client-side version of a known protocol
/// # Example
/// ```
/// use pallas_network::miniprotocols::*;
/// let channel = PROTOCOL_CLIENT | PROTOCOL_N2N_HANDSHAKE;
/// ```
#[deprecated(note = "Use `pallas_primitives::types::network_constant::PROTOCOL_CLIENT` instead")]
pub const PROTOCOL_CLIENT: u16 = pallas_primitives::types::network_constant::PROTOCOL_CLIENT;

/// Bitflag for server-side version of a known protocol
/// # Example
/// ```
/// use pallas_network::miniprotocols::*;
/// let channel = PROTOCOL_SERVER | PROTOCOL_N2N_CHAIN_SYNC;
/// ```
#[deprecated(note = "Use `pallas_primitives::types::network_constant::PROTOCOL_SERVER` instead")]
pub const PROTOCOL_SERVER: u16 = pallas_primitives::types::network_constant::PROTOCOL_SERVER;

/// Protocol channel number for node-to-node handshakes
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2N_HANDSHAKE` instead"
)]
pub const PROTOCOL_N2N_HANDSHAKE: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2N_HANDSHAKE;

/// Protocol channel number for node-to-node chain-sync
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2N_CHAIN_SYNC` instead"
)]
pub const PROTOCOL_N2N_CHAIN_SYNC: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2N_CHAIN_SYNC;

/// Protocol channel number for node-to-node block-fetch
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2N_BLOCK_FETCH` instead"
)]
pub const PROTOCOL_N2N_BLOCK_FETCH: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2N_BLOCK_FETCH;

/// Protocol channel number for node-to-node tx-submission
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2N_TX_SUBMISSION` instead"
)]
pub const PROTOCOL_N2N_TX_SUBMISSION: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2N_TX_SUBMISSION;

/// Protocol channel number for node-to-node Keep-alive
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2N_KEEP_ALIVE` instead"
)]
pub const PROTOCOL_N2N_KEEP_ALIVE: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2N_KEEP_ALIVE;

/// Protocol channel number for node-to-node Peer-sharing
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2N_PEER_SHARING` instead"
)]
pub const PROTOCOL_N2N_PEER_SHARING: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2N_PEER_SHARING;

/// Protocol channel number for node-to-client handshakes
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2C_HANDSHAKE` instead"
)]
pub const PROTOCOL_N2C_HANDSHAKE: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2C_HANDSHAKE;

/// Protocol channel number for node-to-client chain-sync
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2C_CHAIN_SYNC` instead"
)]
pub const PROTOCOL_N2C_CHAIN_SYNC: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2C_CHAIN_SYNC;

/// Protocol channel number for node-to-client tx-submission
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2C_TX_SUBMISSION` instead"
)]
pub const PROTOCOL_N2C_TX_SUBMISSION: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2C_TX_SUBMISSION;

/// Protocol channel number for node-to-client state queries
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2C_STATE_QUERY` instead"
)]
pub const PROTOCOL_N2C_STATE_QUERY: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2C_STATE_QUERY;

/// Protocol channel number for node-to-client mempool monitor
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2C_TX_MONITOR` instead"
)]
pub const PROTOCOL_N2C_TX_MONITOR: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2C_TX_MONITOR;

/// Protocol channel number for node-to-client local message submission
/// This protocol is available only on the DMQ node.
// TODO: use the final mini-protocol number once available
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2C_MSG_SUBMISSION` instead"
)]
pub const PROTOCOL_N2C_MSG_SUBMISSION: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2C_MSG_SUBMISSION;

/// Protocol channel number for node-to-client local message notification
/// This protocol is available only on the DMQ node.
// TODO: use the final mini-protocol number once available
#[deprecated(
    note = "Use `pallas_primitives::types::network_constant::PROTOCOL_N2C_MSG_NOTIFICATION` instead"
)]
pub const PROTOCOL_N2C_MSG_NOTIFICATION: u16 =
    pallas_primitives::types::network_constant::PROTOCOL_N2C_MSG_NOTIFICATION;

/// A point within a chain
#[deprecated(note = "Use `pallas_primitives::types::point::Point` instead")]
pub use pallas_primitives::types::point::Point;
