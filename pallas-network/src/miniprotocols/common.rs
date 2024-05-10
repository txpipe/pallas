use std::fmt::Debug;

use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

/// Well-known magic for testnet
pub const TESTNET_MAGIC: u64 = 1097911063;

/// Well-known magic for mainnet
pub const MAINNET_MAGIC: u64 = 764824073;

/// Well-known magic for preview
pub const PREVIEW_MAGIC: u64 = 2;

/// Well-known magic for preprod
pub const PREPROD_MAGIC: u64 = 1;

/// Alias for PREPROD_MAGIC
pub const PRE_PRODUCTION_MAGIC: u64 = 1;

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

/// Protocol channel number for node-to-client handshakes
pub const PROTOCOL_N2C_HANDSHAKE: u16 = 0;

/// Protocol channel number for node-to-client chain-sync
pub const PROTOCOL_N2C_CHAIN_SYNC: u16 = 5;

/// Protocol channel number for node-to-client tx-submission
pub const PROTOCOL_N2C_TX_SUBMISSION: u16 = 6;

// Protocol channel number for node-to-client state queries
pub const PROTOCOL_N2C_STATE_QUERY: u16 = 7;

/// A point within a chain
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum Point {
    Origin,
    Specific(u64, Vec<u8>),
}

impl Point {
    pub fn slot_or_default(&self) -> u64 {
        match self {
            Point::Origin => 0,
            Point::Specific(slot, _) => *slot,
        }
    }

    pub fn hash_or_default(&self) -> String {
        match self {
            Point::Origin => String::new(),
            Point::Specific(_, hash) => hex::encode(hash),
        }
    }
}

impl Debug for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Origin => write!(f, "Origin"),
            Self::Specific(arg0, arg1) => write!(f, "({}, {})", arg0, hex::encode(arg1)),
        }
    }
}

impl Point {
    pub fn new(slot: u64, hash: Vec<u8>) -> Self {
        Point::Specific(slot, hash)
    }
}

impl Encode<()> for Point {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Point::Origin => e.array(0)?,
            Point::Specific(slot, hash) => e.array(2)?.u64(*slot)?.bytes(hash)?,
        };

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for Point {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let size = d.array()?;

        match size {
            Some(0) => Ok(Point::Origin),
            Some(2) => {
                let slot = d.u64()?;
                let hash = d.bytes()?;
                Ok(Point::Specific(slot, Vec::from(hash)))
            }
            _ => Err(decode::Error::message(
                "can't decode Point from array of size",
            )),
        }
    }
}
