use std::collections::BTreeMap;
use std::fmt::Debug;

use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, decode, encode};

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
pub const PROTOCOL_CLIENT: u16 = 0x0;

/// Bitflag for server-side version of a known protocol
pub const PROTOCOL_SERVER: u16 = 0x8000;

/// Protocol channel number for node-to-client handshakes
pub const PROTOCOL_N2C_HANDSHAKE: u16 = 0;

/// Protocol channel number for node-to-client chain-sync
pub const PROTOCOL_N2C_CHAIN_SYNC: u16 = 5;

/// Protocol channel number for node-to-client tx-submission
pub const PROTOCOL_N2C_TX_SUBMISSION: u16 = 6;

/// Protocol channel number for node-to-client state queries
pub const PROTOCOL_N2C_STATE_QUERY: u16 = 7;

/// Protocol channel number for node-to-client mempool monitor
pub const PROTOCOL_N2C_TX_MONITOR: u16 = 9;

/// Protocol channel number for node-to-client local message submission
/// This protocol is available only on the DMQ node.
// TODO: use the final mini-protocol number once available
pub const PROTOCOL_N2C_MSG_SUBMISSION: u16 = 1;

/// Protocol channel number for node-to-client local message notification
/// This protocol is available only on the DMQ node.
// TODO: use the final mini-protocol number once available
pub const PROTOCOL_N2C_MSG_NOTIFICATION: u16 = 2;

/// A point within a chain
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum Point {
    /// The genesis (origin) point of the chain.
    Origin,
    /// A specific point identified by slot number and block header hash.
    Specific(u64, Vec<u8>),
}

impl Point {
    /// Returns the slot number, or 0 for the origin point.
    pub fn slot_or_default(&self) -> u64 {
        match self {
            Point::Origin => 0,
            Point::Specific(slot, _) => *slot,
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
    /// Creates a new specific point from a slot number and block header hash.
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

// ---------------------------------------------------------------------------
// Leios shared wire primitives
//
// These types are shared across the Leios mini-protocols (`leiosnotify`,
// `leiosfetch`). They live here next to [`Point`] following the same convention
// used for other cross-protocol types.
// ---------------------------------------------------------------------------

/// Reference to an Endorser Block, encoded as a `[slot, eb_hash]` point.
///
/// Wire-compatible with the `pcommon.Point` used by the Go reference
/// implementation for EB references.
pub type EbId = Point;

/// A pre-encoded CBOR item embedded verbatim into a message.
///
/// The Leios mini-protocols carry heavy payloads (EB bodies, transactions,
/// votes) as raw CBOR spliced directly into the message array — the equivalent
/// of Go's `cbor.RawMessage`. Note this is **not** the tag-24 ("CBOR-in-CBOR")
/// byte-string wrapping used by [`super::blockfetch::Message::Block`] or
/// chain-sync headers: the bytes are the encoded item itself, written and read
/// in place. Structural decoding of the payload is deferred to higher layers
/// (e.g. `pallas-primitives`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCbor(pub Vec<u8>);

impl<C> Encode<C> for RawCbor {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.writer_mut()
            .write_all(&self.0)
            .map_err(encode::Error::write)
    }
}

impl<'b, C> Decode<'b, C> for RawCbor {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut C) -> Result<Self, decode::Error> {
        let all = d.input();
        let start = d.position();
        d.skip()?;
        let end = d.position();

        Ok(RawCbor(all[start..end].to_vec()))
    }
}

/// Raw CBOR of an Endorser Block body (`omap<hash32, uint16>`).
pub type EndorserBlockCbor = RawCbor;

/// Raw CBOR of a single transaction.
pub type TxCbor = RawCbor;

/// Raw CBOR of a single Leios vote (persistent or non-persistent).
pub type VoteCbor = RawCbor;

/// Raw CBOR of a Leios certificate.
pub type CertCbor = RawCbor;

/// A transaction-subset selector for [`super::leiosfetch`] block-txs requests.
///
/// Each key indexes a 64-transaction window (window `n` covers txs
/// `64*n .. 64*n+63`); each set bit in the `u64` value selects a transaction
/// within that window.
///
/// **Wire note:** this *must* serialize as an indefinite-length CBOR map
/// (`0xbf … 0xff`). The Leios prototype rejects a definite-length map and resets
/// the connection. Decoding accepts either form (a [`BTreeMap`] keeps key order
/// deterministic).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bitmaps(pub BTreeMap<u16, u64>);

impl Encode<()> for Bitmaps {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.begin_map()?;
        for (k, v) in &self.0 {
            e.u16(*k)?;
            e.u64(*v)?;
        }
        e.end()?;

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for Bitmaps {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        // minicbor's `BTreeMap` decoder transparently handles both definite and
        // indefinite-length maps.
        Ok(Bitmaps(d.decode()?))
    }
}

#[cfg(test)]
mod leios_tests {
    use super::*;
    use pallas_codec::minicbor;

    #[test]
    fn bitmaps_encode_is_indefinite() {
        let mut m = BTreeMap::new();
        m.insert(0u16, 0xffff_ffff_ffff_ffffu64);
        m.insert(1u16, 0x0000_0000_0001_0000u64);
        let bm = Bitmaps(m);

        let bytes = minicbor::to_vec(&bm).unwrap();
        // indefinite-length map marker, terminated by break
        assert_eq!(bytes[0], 0xbf, "bitmaps must use an indefinite-length map");
        assert_eq!(*bytes.last().unwrap(), 0xff, "must be break-terminated");

        let back: Bitmaps = minicbor::decode(&bytes).unwrap();
        assert_eq!(back, bm);
    }

    #[test]
    fn bitmaps_decode_accepts_definite() {
        // A definite-length map { 0: 1 } encoded as 0xa1 00 01
        let definite = [0xa1u8, 0x00, 0x01];
        let back: Bitmaps = minicbor::decode(&definite).unwrap();
        assert_eq!(back.0.get(&0), Some(&1u64));
    }

    #[test]
    fn raw_cbor_embeds_verbatim() {
        // Encode a wrapper [1, <raw>] where <raw> is a pre-encoded array [1,2,3].
        let inner = minicbor::to_vec([1u32, 2, 3]).unwrap();
        let raw = RawCbor(inner.clone());

        let mut buf = Vec::new();
        let mut enc = Encoder::new(&mut buf);
        enc.array(2).unwrap();
        enc.u16(1).unwrap();
        enc.encode(&raw).unwrap();

        // The raw item must appear byte-for-byte (not byte-string wrapped).
        let header = [0x82u8, 0x01]; // array(2), 1
        assert_eq!(&buf[..2], &header);
        assert_eq!(&buf[2..], inner.as_slice());

        // And it round-trips back to the same bytes.
        let mut dec = Decoder::new(&buf);
        dec.array().unwrap();
        let _: u16 = dec.u16().unwrap();
        let back: RawCbor = dec.decode().unwrap();
        assert_eq!(back, raw);
    }
}
