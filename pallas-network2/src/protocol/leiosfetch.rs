//! LeiosFetch mini-protocol implementation.
//!
//! Client-pull protocol for fetching Endorser Block (EB) bodies and their
//! transactions (selected by a compact bitmap), discovered via
//! [`super::leiosnotify`]. The client issues one request and the server replies
//! with the matching response, returning to idle.
//!
//! Wire format and state machine follow the authoritative `leios-fetch` CDDL on
//! the `leios-prototype` branch of cardano-blueprint (protocol id 19), which is
//! the network spec of record while CIP-0164's network chapter stabilises.

use std::collections::BTreeMap;

use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, decode, encode};

use super::{EbId, Error, RawCbor};

/// Protocol channel number for node-to-node leios-fetch.
pub const CHANNEL_ID: u16 = 19;

/// Raw CBOR of an Endorser Block body (`{ hash32 => word32 }`).
pub type EndorserBlockCbor = RawCbor;

/// Raw CBOR of a single transaction.
pub type TxCbor = RawCbor;

/// A transaction-subset selector for leios-fetch block-txs requests.
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

impl Bitmaps {
    /// Selects the first `count` transactions of an EB (txs `0..count`).
    ///
    /// Transactions are addressed within 64-tx windows; tx `i` of a window is the
    /// **most-significant** bit (tx 0 → bit 63), matching the wire convention.
    /// `count == 0` selects nothing.
    pub fn all(count: usize) -> Self {
        let mut m = BTreeMap::new();
        let mut remaining = count;
        let mut window = 0u16;
        while remaining > 0 {
            let bits = remaining.min(64);
            // Top `bits` bits set (MSB-first: tx 0 of the window is bit 63).
            let mask = if bits == 64 {
                u64::MAX
            } else {
                !((1u64 << (64 - bits)) - 1)
            };
            m.insert(window, mask);
            remaining -= bits;
            window += 1;
        }
        Bitmaps(m)
    }

    /// Selects the transactions at the given sequence indices within an EB, using
    /// the same MSB-first 64-tx window convention as [`Bitmaps::all`].
    pub fn from_indices(indices: impl IntoIterator<Item = usize>) -> Self {
        let mut m = BTreeMap::new();
        for offset in indices {
            let window = (offset / 64) as u16;
            let bit = 63 - (offset % 64);
            *m.entry(window).or_insert(0) |= 1u64 << bit;
        }
        Bitmaps(m)
    }
}

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

/// A leios-fetch mini-protocol message.
#[derive(Debug, Clone)]
pub enum Message {
    /// Client requests a complete EB body.
    BlockRequest(EbId),
    /// Server delivers an EB body (raw CBOR).
    Block(EndorserBlockCbor),
    /// Client requests a subset of an EB's transactions, selected by bitmap.
    BlockTxsRequest(EbId, Bitmaps),
    /// Server delivers transactions for an EB, echoing the request's point and
    /// bitmaps: `[3, point, bitmaps, tx_list]`.
    BlockTxs {
        /// Echoed EB point.
        point: EbId,
        /// Echoed bitmap selector.
        bitmaps: Bitmaps,
        /// The requested transactions, as raw CBOR.
        txs: Vec<TxCbor>,
    },
    /// Client terminates the protocol.
    Done,
}

/// A response delivered by the server, retained (with the EB it answers) in the
/// idle state until the consumer drains it (mirrors the chain-sync `Data` idiom).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// An EB body (raw CBOR).
    Block(EndorserBlockCbor),
    /// Transactions delivered for an EB. (The echoed point/bitmaps from the wire
    /// are dropped — the EB is carried alongside the response by the state.)
    BlockTxs {
        /// The delivered transactions.
        txs: Vec<TxCbor>,
    },
}

/// State machine for the leios-fetch mini-protocol.
///
/// The `Awaiting*` states retain the request parameters so a responder can serve
/// them; the `Idle` state retains the delivered response — paired with the
/// [`EbId`] it answers — until the consumer drains it.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Client has agency; can issue a request or finish.
    Idle(Option<(EbId, Response)>),
    /// Server has agency; will deliver the requested EB body.
    AwaitingBlock(EbId),
    /// Server has agency; will deliver transactions for the requested EB.
    AwaitingBlockTxs(EbId, Bitmaps),
    /// The protocol has terminated.
    Done,
}

impl Default for State {
    fn default() -> Self {
        State::Idle(None)
    }
}

impl State {
    /// Applies a message to the current state, returning the new state.
    pub fn apply(&self, msg: &Message) -> Result<Self, Error> {
        match self {
            State::Idle(_) => match msg {
                Message::BlockRequest(p) => Ok(State::AwaitingBlock(p.clone())),
                Message::BlockTxsRequest(p, b) => Ok(State::AwaitingBlockTxs(p.clone(), b.clone())),
                Message::Done => Ok(State::Done),
                _ => Err(Error::InvalidOutbound),
            },
            State::AwaitingBlock(eb) => match msg {
                Message::Block(b) => {
                    Ok(State::Idle(Some((eb.clone(), Response::Block(b.clone())))))
                }
                _ => Err(Error::InvalidInbound),
            },
            State::AwaitingBlockTxs(eb, _) => match msg {
                // The wire form echoes point/bitmaps; we keep only the txs and pair
                // them with the EB from our request state.
                Message::BlockTxs { txs, .. } => Ok(State::Idle(Some((
                    eb.clone(),
                    Response::BlockTxs { txs: txs.clone() },
                )))),
                _ => Err(Error::InvalidInbound),
            },
            State::Done => Err(Error::InvalidOutbound),
        }
    }

    /// Takes any pending response (with the EB it answers), leaving the state
    /// idle. Returns `None` if there is nothing to drain or the protocol is not
    /// idle.
    pub fn drain(&mut self) -> Option<(EbId, Response)> {
        match self {
            State::Idle(r) => r.take(),
            _ => None,
        }
    }
}

impl Encode<()> for Message {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::BlockRequest(point) => {
                e.array(2)?.u16(0)?;
                e.encode(point)?;
            }
            Message::Block(block) => {
                e.array(2)?.u16(1)?;
                e.encode(block)?;
            }
            Message::BlockTxsRequest(point, bitmaps) => {
                e.array(3)?.u16(2)?;
                e.encode(point)?;
                e.encode(bitmaps)?;
            }
            Message::BlockTxs {
                point,
                bitmaps,
                txs,
            } => {
                e.array(4)?.u16(3)?;
                e.encode(point)?;
                e.encode(bitmaps)?;
                e.encode(txs)?;
            }
            Message::Done => {
                e.array(1)?.u16(9)?;
            }
        }

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for Message {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => Ok(Message::BlockRequest(d.decode()?)),
            1 => Ok(Message::Block(d.decode()?)),
            2 => {
                let point = d.decode()?;
                let bitmaps = d.decode()?;
                Ok(Message::BlockTxsRequest(point, bitmaps))
            }
            3 => {
                let point = d.decode()?;
                let bitmaps = d.decode()?;
                let txs = d.decode()?;
                Ok(Message::BlockTxs {
                    point,
                    bitmaps,
                    txs,
                })
            }
            9 => Ok(Message::Done),
            _ => Err(decode::Error::message(
                "unknown variant for leiosfetch message",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "blueprint")]
    use crate::protocol::cddl;
    use crate::protocol::cddl::conforms;
    use crate::protocol::{Point, RawCbor};
    use pallas_codec::minicbor;
    use std::collections::BTreeMap;

    fn point() -> EbId {
        Point::Specific(99, vec![0xCD; 32])
    }

    fn bitmaps() -> Bitmaps {
        let mut m = BTreeMap::new();
        m.insert(0u16, 0xff00u64);
        Bitmaps(m)
    }

    fn raw(bytes: [u8; 3]) -> RawCbor {
        RawCbor(minicbor::to_vec(bytes).unwrap())
    }

    fn reencode(msg: &Message) -> Vec<u8> {
        minicbor::to_vec(msg).unwrap()
    }

    fn roundtrip_eq(msg: &Message) {
        let bytes = reencode(msg);
        let back: Message = minicbor::decode(&bytes).unwrap();
        assert_eq!(reencode(&back), bytes);
    }

    #[test]
    fn message_roundtrips() {
        roundtrip_eq(&Message::BlockRequest(point()));
        roundtrip_eq(&Message::Block(RawCbor(
            minicbor::to_vec([1u8, 2]).unwrap(),
        )));
        roundtrip_eq(&Message::BlockTxsRequest(point(), bitmaps()));
        roundtrip_eq(&Message::BlockTxs {
            point: point(),
            bitmaps: bitmaps(),
            txs: vec![raw([8, 8, 8])],
        });
        roundtrip_eq(&Message::Done);
    }

    #[test]
    fn block_txs_roundtrip() {
        let msg = Message::BlockTxs {
            point: point(),
            bitmaps: bitmaps(),
            txs: vec![raw([8, 8, 8])],
        };
        let bytes = reencode(&msg);
        // envelope is a 4-element array: [tag=3, point, bitmaps, txs]
        assert_eq!(bytes[0], 0x84);
        let back: Message = minicbor::decode(&bytes).unwrap();
        assert!(matches!(back, Message::BlockTxs { .. }));
    }

    #[test]
    fn state_transitions_and_drain() {
        assert_eq!(State::default(), State::Idle(None));
        assert_eq!(
            State::Idle(None)
                .apply(&Message::BlockRequest(point()))
                .unwrap(),
            State::AwaitingBlock(point())
        );

        let mut idle = State::AwaitingBlock(point())
            .apply(&Message::Block(raw([1, 2, 3])))
            .unwrap();
        assert_eq!(
            idle,
            State::Idle(Some((point(), Response::Block(raw([1, 2, 3])))))
        );
        assert_eq!(
            idle.drain(),
            Some((point(), Response::Block(raw([1, 2, 3]))))
        );
        assert_eq!(idle, State::Idle(None));
        assert_eq!(idle.drain(), None);
    }

    #[test]
    fn awaiting_states_retain_request_params() {
        assert_eq!(
            State::Idle(None)
                .apply(&Message::BlockTxsRequest(point(), bitmaps()))
                .unwrap(),
            State::AwaitingBlockTxs(point(), bitmaps())
        );
    }

    #[test]
    fn illegal_transition_errors() {
        assert!(matches!(
            State::Idle(None).apply(&Message::Block(raw([1, 2, 3]))),
            Err(Error::InvalidOutbound)
        ));
        assert!(matches!(
            State::AwaitingBlock(point()).apply(&Message::BlockRequest(point())),
            Err(Error::InvalidInbound)
        ));
    }

    #[test]
    fn bitmaps_all_is_msb_first() {
        assert_eq!(Bitmaps::all(0).0.len(), 0);
        assert_eq!(Bitmaps::all(1).0.get(&0), Some(&(1u64 << 63)));
        assert_eq!(Bitmaps::all(64).0.get(&0), Some(&u64::MAX));
        let two = Bitmaps::all(65);
        assert_eq!(two.0.get(&0), Some(&u64::MAX));
        assert_eq!(two.0.get(&1), Some(&(1u64 << 63)));
    }

    #[test]
    fn bitmaps_from_indices_is_msb_first() {
        // tx 0 -> window 0 bit 63; tx 65 -> window 1 bit 62.
        let b = Bitmaps::from_indices([0usize, 65]);
        assert_eq!(b.0.get(&0), Some(&(1u64 << 63)));
        assert_eq!(b.0.get(&1), Some(&(1u64 << 62)));
    }

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

    // --- CBOR-vs-CDDL conformance (run with `--features blueprint`) ---
    //
    // Each `conforms!` below emits one `#[test]` that encodes a sample message
    // with our `Encode` impl and validates the bytes against the vendored
    // cardano-blueprint leios-fetch CDDL (via the shared `cddl` helper),
    // so a spec change (tag, arity, the bitmaps shape) fails the matching test.
    // The EB body / txs are opaque `RawCbor` here, so they are validated as `any`.

    /// Turns the vendored leios-fetch CDDL into a schema cddl-rs can parse. On top
    /// of the shared preprocessing this relaxes the opaque sub-structures (the EB
    /// body and `tx.tx`) to `any`, since they are raw CBOR in our codec.
    #[cfg(feature = "blueprint")]
    fn self_contained() -> String {
        let body = cddl::preprocess(include_str!(
            "../../../cardano-blueprint/src/network/node-to-node/leios-fetch/messages.cddl"
        ))
        .replace(
            "endorser_block = { * hash => word32 }",
            "endorser_block = any",
        )
        .replace("tx.tx", "tx_tx");
        format!("{body}\n{}tx_tx = any\n", cddl::BASE_PRELUDE)
    }

    conforms!(
        block_request_conforms,
        self_contained,
        "msgLeiosBlockRequest",
        Message::BlockRequest(point())
    );
    conforms!(
        block_conforms,
        self_contained,
        "msgLeiosBlock",
        Message::Block(raw([1, 2, 3]))
    );
    conforms!(
        block_txs_request_conforms,
        self_contained,
        "msgLeiosBlockTxsRequest",
        Message::BlockTxsRequest(point(), Bitmaps::all(8))
    );
    conforms!(
        block_txs_conforms,
        self_contained,
        "msgLeiosBlockTxs",
        Message::BlockTxs {
            point: point(),
            bitmaps: Bitmaps::all(8),
            txs: vec![raw([1, 2, 3])],
        }
    );
    conforms!(
        done_conforms,
        self_contained,
        "msgClientDone",
        Message::Done
    );
}
