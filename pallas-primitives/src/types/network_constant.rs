//! Network constants

/// Well-known magic for testnet
pub const TESTNET_MAGIC: u64 = 1097911063;
pub const TESTNET_NETWORK_ID: u64 = 0;

/// Well-known magic for mainnet
pub const MAINNET_MAGIC: u64 = 764824073;
pub const MAINNET_NETWORK_ID: u64 = 1;

/// Well-known magic for preview
pub const PREVIEW_MAGIC: u64 = 2;
pub const PREVIEW_NETWORK_ID: u64 = 0;

/// Well-known magic for preprod
pub const PREPROD_MAGIC: u64 = 1;

/// Alias for PREPROD_MAGIC
pub const PRE_PRODUCTION_MAGIC: u64 = 1;
pub const PRE_PRODUCTION_NETWORK_ID: u64 = 0;

/// Well-known magic for preprod
pub const SANCHONET_MAGIC: u64 = 4;

/// Bitflag for client-side version of a known protocol
/// # Example
/// ```
/// use pallas_network::miniprotocols::*;
/// let channel = PROTOCOL_CLIENT | PROTOCOL_N2N_HANDSHAKE;
/// ```
pub const PROTOCOL_CLIENT: u16 = 0x0;

/// Bitflag for server-side version of a known protocol
/// # Example
/// ```
/// use pallas_network::miniprotocols::*;
/// let channel = PROTOCOL_SERVER | PROTOCOL_N2N_CHAIN_SYNC;
/// ```
pub const PROTOCOL_SERVER: u16 = 0x8000;

/// Protocol channel number for node-to-node handshakes
pub const PROTOCOL_N2N_HANDSHAKE: u16 = 0;

/// Protocol channel number for node-to-node chain-sync
pub const PROTOCOL_N2N_CHAIN_SYNC: u16 = 2;

/// Protocol channel number for node-to-node block-fetch
pub const PROTOCOL_N2N_BLOCK_FETCH: u16 = 3;

/// Protocol channel number for node-to-node tx-submission
pub const PROTOCOL_N2N_TX_SUBMISSION: u16 = 4;

/// Protocol channel number for node-to-node Keep-alive
pub const PROTOCOL_N2N_KEEP_ALIVE: u16 = 8;

/// Protocol channel number for node-to-node Peer-sharing
pub const PROTOCOL_N2N_PEER_SHARING: u16 = 10;

/// Protocol channel number for node-to-client handshakes
pub const PROTOCOL_N2C_HANDSHAKE: u16 = 0;

/// Protocol channel number for node-to-client chain-sync
pub const PROTOCOL_N2C_CHAIN_SYNC: u16 = 5;

/// Protocol channel number for node-to-client tx-submission
pub const PROTOCOL_N2C_TX_SUBMISSION: u16 = 6;

// Protocol channel number for node-to-client state queries
pub const PROTOCOL_N2C_STATE_QUERY: u16 = 7;

// Protocol channel number for node-to-client mempool monitor
pub const PROTOCOL_N2C_TX_MONITOR: u16 = 9;

/// Protocol channel number for node-to-client local message submission
/// This protocol is available only on the DMQ node.
// TODO: use the final mini-protocol number once available
pub const PROTOCOL_N2C_MSG_SUBMISSION: u16 = 1;

/// Protocol channel number for node-to-client local message notification
/// This protocol is available only on the DMQ node.
// TODO: use the final mini-protocol number once available
pub const PROTOCOL_N2C_MSG_NOTIFICATION: u16 = 2;

/// Protocol value that defines max segment length
pub const MAX_SEGMENT_PAYLOAD_LENGTH: usize = 65535;
