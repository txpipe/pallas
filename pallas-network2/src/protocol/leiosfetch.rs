//! LeiosFetch mini-protocol implementation.
//!
//! Client-pull protocol for fetching Endorser Block (EB) bodies and their
//! transactions (selected by a compact bitmap), discovered via
//! [`super::leiosnotify`]. The client issues one request and the server replies
//! with the matching response, returning to idle.
//!
//! Wire format and state machine follow the authoritative `leios-fetch` CDDL of
//! cardano-blueprint (protocol id 19), which is the network spec of record while
//! CIP-0164's network chapter stabilises. With the `unstable` feature an
//! endorser block is written in tag 24 and read with or without it.

use std::collections::BTreeMap;

#[cfg(feature = "unstable")]
use pallas_codec::minicbor::data::IanaTag;
use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, decode, encode};
use pallas_codec::utils::CborWrap;
#[cfg(feature = "unstable")]
use pallas_codec::utils::{Bytes, TagWrap};

use super::{AnyCbor, EbId, Error};

/// Protocol channel number for node-to-node leios-fetch.
pub const CHANNEL_ID: u16 = 19;

/// Raw CBOR of an Endorser Block body (`{ hash32 => word32 }`).
pub type EndorserBlockCbor = AnyCbor;

/// Raw CBOR of a single transaction.
pub type TxCbor = AnyCbor;

#[cfg(feature = "unstable")]
type Wrapped = TagWrap<Bytes, 24>;

#[cfg(feature = "unstable")]
fn wrap(payload: &AnyCbor) -> Wrapped {
    TagWrap(Bytes::from(payload.raw_bytes().to_vec()))
}

#[cfg(feature = "unstable")]
fn unwrap(wrapped: Wrapped) -> AnyCbor {
    AnyCbor::from_raw_bytes(wrapped.0.into())
}

#[cfg(feature = "unstable")]
fn next_is_wrapped(d: &mut Decoder) -> bool {
    matches!(d.probe().tag(), Ok(tag) if tag == IanaTag::Cbor.tag())
}

/// A transaction-subset selector for leios-fetch block-txs requests.
///
/// Each key indexes a 64-transaction window (window `n` covers txs
/// `64*n .. 64*n+63`); each set bit in the `u64` value selects a transaction
/// within that window.
///
/// **Wire note:** this *must* serialize as an indefinite-length CBOR map
/// (`0xbf … 0xff`). The Leios prototype rejects a definite-length map and resets
/// the connection. Decoding accepts either form (a [`BTreeMap`] keeps key order
/// deterministic), and a window that appears twice keeps its last bitmap.
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
        Ok(Bitmaps(d.decode()?))
    }
}

/// A leios-fetch mini-protocol message.
#[derive(Debug, Clone)]
pub enum Message {
    /// Client terminates the protocol.
    Done,
    /// Client requests a complete EB body.
    BlockRequest(EbId),
    /// Server delivers an EB body (raw CBOR).
    Block(EndorserBlockCbor),
    /// Client requests a subset of an EB's transactions, selected by bitmap.
    BlockTxsRequest(EbId, Bitmaps),
    /// Server delivers the requested transactions of an EB: `[4, tx_list]`.
    BlockTxs {
        /// The requested transactions, as raw CBOR.
        txs: Vec<TxCbor>,
    },
}

/// A response delivered by the server, retained (with the EB it answers) in the
/// idle state until the consumer drains it (mirrors the chain-sync `Data` idiom).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// An EB body (raw CBOR).
    Block(EndorserBlockCbor),
    /// Transactions delivered for an EB.
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
                Message::BlockTxs { txs } => Ok(State::Idle(Some((
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
            Message::Done => {
                e.array(1)?.u16(0)?;
            }
            Message::BlockRequest(point) => {
                e.array(2)?.u16(1)?;
                e.encode(point)?;
            }
            Message::Block(block) => {
                e.array(2)?.u16(2)?;
                #[cfg(feature = "unstable")]
                e.encode(wrap(block))?;
                #[cfg(not(feature = "unstable"))]
                e.encode(block)?;
            }
            Message::BlockTxsRequest(point, bitmaps) => {
                e.array(3)?.u16(3)?;
                e.encode(point)?;
                e.encode(bitmaps)?;
            }
            Message::BlockTxs { txs } => {
                e.array(2)?.u16(4)?;
                e.array(txs.len() as u64)?;
                for tx in txs {
                    e.encode(CborWrap(tx))?;
                }
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
            0 => Ok(Message::Done),
            1 => Ok(Message::BlockRequest(d.decode()?)),
            2 => {
                #[cfg(feature = "unstable")]
                let block = if next_is_wrapped(d) {
                    unwrap(d.decode()?)
                } else {
                    d.decode()?
                };
                #[cfg(not(feature = "unstable"))]
                let block = d.decode()?;
                Ok(Message::Block(block))
            }
            3 => {
                let point = d.decode()?;
                let bitmaps = d.decode()?;
                Ok(Message::BlockTxsRequest(point, bitmaps))
            }
            4 => {
                #[cfg(feature = "unstable")]
                let txs = d.decode::<Vec<Wrapped>>()?.into_iter().map(unwrap);
                #[cfg(not(feature = "unstable"))]
                let txs = d
                    .decode::<Vec<CborWrap<TxCbor>>>()?
                    .into_iter()
                    .map(CborWrap::unwrap);
                Ok(Message::BlockTxs { txs: txs.collect() })
            }
            _ => Err(decode::Error::message(
                "unknown variant for leiosfetch message",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Point;
    #[cfg(feature = "blueprint")]
    use crate::protocol::cddl;
    use crate::protocol::cddl::conforms;
    use pallas_codec::minicbor;
    #[cfg(feature = "unstable")]
    use pallas_crypto::hash::Hasher;
    use std::collections::BTreeMap;

    fn point() -> EbId {
        Point::Specific(99, vec![0xCD; 32])
    }

    fn bitmaps() -> Bitmaps {
        let mut m = BTreeMap::new();
        m.insert(0u16, 0xff00u64);
        Bitmaps(m)
    }

    fn raw(bytes: [u8; 3]) -> AnyCbor {
        AnyCbor::from_raw_bytes(minicbor::to_vec(bytes).unwrap())
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
        roundtrip_eq(&Message::Block(AnyCbor::from_raw_bytes(
            minicbor::to_vec([1u8, 2]).unwrap(),
        )));
        roundtrip_eq(&Message::BlockTxsRequest(point(), bitmaps()));
        roundtrip_eq(&Message::BlockTxs {
            txs: vec![raw([8, 8, 8])],
        });
        roundtrip_eq(&Message::Done);
        assert_eq!(reencode(&Message::Done), [0x81, 0x00]);
    }

    fn decode_captured(bytes: &[u8]) -> Message {
        let msg: Message = minicbor::decode(bytes).expect("captured message decodes");
        assert_eq!(reencode(&msg), bytes, "re-encodes byte equal");
        msg
    }

    fn captured_point() -> EbId {
        Point::Specific(
            529,
            hex::decode("17c497727d273e8dbd0bc188752ef8a7334f5fac6dc077324424a5410cc77fc0")
                .unwrap(),
        )
    }

    const CAPTURED_BLOCK: &[u8] =
        include_bytes!("../../../test_data/leios-rc-2026w42/fetch-block.cbor");

    fn wrapped_block() -> Vec<u8> {
        let body = &CAPTURED_BLOCK[2..];
        assert_eq!(body.len(), 0x09b1, "fixture precondition");
        [&[0x82, 0x02, 0xd8, 0x18, 0x59, 0x09, 0xb1][..], body].concat()
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn a_block_bare_or_cbor_in_cbor_holds_the_announced_endorser_block() {
        let Point::Specific(_, announced) = captured_point() else {
            panic!("fixture precondition");
        };

        for bytes in [CAPTURED_BLOCK.to_vec(), wrapped_block()] {
            match minicbor::decode(&bytes).expect("the block decodes") {
                Message::Block(eb) => {
                    assert_eq!(Hasher::<256>::hash(eb.raw_bytes()).as_ref(), announced)
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn a_block_is_written_cbor_in_cbor() {
        let eb = AnyCbor::from_raw_bytes(CAPTURED_BLOCK[2..].to_vec());
        assert_eq!(reencode(&Message::Block(eb)), wrapped_block());
    }

    #[cfg(not(feature = "unstable"))]
    #[test]
    fn a_block_is_written_and_read_as_one_item() {
        match decode_captured(CAPTURED_BLOCK) {
            Message::Block(eb) => assert_eq!(eb.raw_bytes(), &CAPTURED_BLOCK[2..]),
            other => panic!("{other:?}"),
        }

        let tagged = wrapped_block();
        match decode_captured(&tagged) {
            Message::Block(eb) => assert_eq!(eb.raw_bytes(), &tagged[2..]),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_block_in_another_tag_is_held_as_one_item() {
        let block = [0x82, 0x02, 0xd9, 0x01, 0x02, 0x42, 0x01, 0x02];

        match minicbor::decode(&block).expect("the block decodes") {
            Message::Block(eb) => assert_eq!(eb.raw_bytes(), &block[2..]),
            other => panic!("{other:?}"),
        }
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn a_wrapped_payload_is_held_whole() {
        // Each tag 24 byte string holds two CBOR items, 0x01 and 0x02.
        let block = [0x82, 0x02, 0xd8, 0x18, 0x42, 0x01, 0x02];
        let block_txs = [0x82, 0x04, 0x81, 0xd8, 0x18, 0x42, 0x01, 0x02];

        match minicbor::decode(&block).expect("the block decodes") {
            Message::Block(eb) => assert_eq!(eb.raw_bytes(), [0x01, 0x02]),
            other => panic!("{other:?}"),
        }
        match minicbor::decode(&block_txs).expect("the transactions decode") {
            Message::BlockTxs { txs } => assert_eq!(txs[0].raw_bytes(), [0x01, 0x02]),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn captured_messages_decode_by_their_numbers() {
        match decode_captured(include_bytes!(
            "../../../test_data/leios-rc-2026w42/fetch-block-request.cbor"
        )) {
            Message::BlockRequest(point) => assert_eq!(point, captured_point()),
            other => panic!("{other:?}"),
        }

        match decode_captured(include_bytes!(
            "../../../test_data/leios-rc-2026w42/fetch-block-txs-request.cbor"
        )) {
            Message::BlockTxsRequest(point, bitmaps) => {
                assert_eq!(point, captured_point());
                assert_eq!(bitmaps, Bitmaps::all(67));
            }
            other => panic!("{other:?}"),
        }

        match decode_captured(include_bytes!(
            "../../../test_data/leios-rc-2026w42/fetch-block-txs.cbor"
        )) {
            Message::BlockTxs { txs } => assert_eq!(txs.len(), 67),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn captured_block_txs_hold_block_transactions() {
        let msg: Message = minicbor::decode(include_bytes!(
            "../../../test_data/leios-rc-2026w42/fetch-block-txs.cbor"
        ))
        .unwrap();
        let Message::BlockTxs { txs } = msg else {
            panic!("{msg:?}");
        };
        for tx in &txs {
            let bytes = tx.raw_bytes();
            assert_eq!(bytes[0], 0x84, "a four element block transaction");
            assert_eq!(bytes[bytes.len() - 1], 0xf5, "is_valid last");
        }
    }

    #[test]
    fn block_txs_roundtrip() {
        let msg = Message::BlockTxs {
            txs: vec![raw([8, 8, 8])],
        };
        let bytes = reencode(&msg);
        assert_eq!(bytes[..2], [0x82, 0x04]);
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
    fn done_ends_the_protocol_only_from_idle() {
        assert_eq!(
            State::Idle(None).apply(&Message::Done).unwrap(),
            State::Done
        );
        assert!(matches!(
            State::AwaitingBlock(point()).apply(&Message::Done),
            Err(Error::InvalidInbound)
        ));
    }

    #[test]
    fn block_txs_answer_only_a_block_txs_request() {
        let txs = vec![raw([4, 5, 6])];
        assert_eq!(
            State::AwaitingBlockTxs(point(), bitmaps())
                .apply(&Message::BlockTxs { txs: txs.clone() })
                .unwrap(),
            State::Idle(Some((point(), Response::BlockTxs { txs: txs.clone() })))
        );
        assert!(matches!(
            State::AwaitingBlock(point()).apply(&Message::BlockTxs { txs }),
            Err(Error::InvalidInbound)
        ));
    }

    #[test]
    fn bitmaps_from_indices_put_each_offset_in_its_64_wide_window() {
        let b = Bitmaps::from_indices([63usize, 64, 200]);
        assert_eq!(
            b.0,
            BTreeMap::from([(0u16, 1u64), (1, 1u64 << 63), (3, 1u64 << 55)])
        );
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

    #[test]
    fn bitmaps_with_distinct_windows_decode_in_either_length_form() {
        let indefinite = [0xbfu8, 0x00, 0x01, 0x01, 0x02, 0xff];
        let definite = [0xa2u8, 0x00, 0x01, 0x01, 0x02];
        for bytes in [&indefinite[..], &definite[..]] {
            let back: Bitmaps = minicbor::decode(bytes).unwrap();
            assert_eq!(back.0.into_iter().collect::<Vec<_>>(), [(0, 1), (1, 2)]);
        }
    }

    #[test]
    fn a_request_naming_one_window_twice_keeps_its_last_bitmap() {
        let captured =
            include_bytes!("../../../test_data/leios-rc-2026w42/fetch-block-txs-request.cbor");
        assert_eq!(captured[51], 0x01, "the key of the second window");
        let mut twice = captured.to_vec();
        twice[51] = 0x00;

        match minicbor::decode::<Message>(&twice).expect("the request decodes") {
            Message::BlockTxsRequest(point, bitmaps) => {
                assert_eq!(point, captured_point());
                assert_eq!(
                    bitmaps.0.into_iter().collect::<Vec<_>>(),
                    [(0, 0xe000_0000_0000_0000)]
                );
            }
            other => panic!("{other:?}"),
        }

        for bytes in [
            &[0xbfu8, 0x00, 0x01, 0x00, 0x02, 0xff][..],
            &[0xa2u8, 0x00, 0x01, 0x00, 0x02],
        ] {
            let back: Bitmaps = minicbor::decode(bytes).expect("the bitmaps decode");
            assert_eq!(back.0.into_iter().collect::<Vec<_>>(), [(0, 2)]);
        }
    }

    // --- CBOR-vs-CDDL conformance (run with `--features blueprint`) ---
    //
    // Each `conforms!` below emits one `#[test]` that encodes a sample message
    // with our `Encode` impl and validates the bytes against the vendored
    // cardano-blueprint leios-fetch CDDL (via the shared `cddl` helper),
    // so a spec change (tag, arity, the bitmaps shape) fails the matching test.
    // The EB body / txs are opaque raw CBOR here, so they are validated as `any`.

    /// Turns the vendored leios-fetch CDDL into a schema cddl-rs can parse. On top
    /// of the shared preprocessing this relaxes the opaque sub-structures (the EB
    /// body and the included Dijkstra `block_transaction`) to `any`, since the
    /// codec holds them as raw CBOR.
    #[cfg(feature = "blueprint")]
    fn self_contained() -> String {
        let body = cddl::preprocess(include_str!(
            "../../../cardano-blueprint/src/network/node-to-node/leios-fetch/messages.cddl"
        ))
        .replace(
            "endorser_block = { + hash => word32 }",
            "endorser_block = any",
        )
        .replace("dijkstra.block_transaction", "block_transaction");
        format!("{body}\n{}block_transaction = any\n", cddl::BASE_PRELUDE)
    }

    /// The [`self_contained`] schema with the endorser block in tag 24.
    #[cfg(all(feature = "blueprint", feature = "unstable"))]
    fn tagged_self_contained() -> String {
        self_contained().replace(
            "msgLeiosBlock                  = [2, endorser_block]",
            "msgLeiosBlock = [2, #6.24(bytes .cbor endorser_block)]",
        )
    }

    conforms!(
        block_request_conforms,
        self_contained,
        "msgLeiosBlockRequest",
        Message::BlockRequest(point())
    );
    #[cfg(not(feature = "unstable"))]
    conforms!(
        block_conforms,
        self_contained,
        "msgLeiosBlock",
        Message::Block(raw([1, 2, 3]))
    );
    #[cfg(feature = "unstable")]
    conforms!(
        block_conforms,
        tagged_self_contained,
        "msgLeiosBlock",
        Message::Block(AnyCbor::from_raw_bytes(CAPTURED_BLOCK[2..].to_vec()))
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
            txs: vec![raw([1, 2, 3])],
        }
    );
    conforms!(done_conforms, self_contained, "msgDone", Message::Done);

    #[cfg(all(feature = "blueprint", feature = "unstable"))]
    #[test]
    fn a_bare_block_does_not_conform() {
        cddl::assert_does_not_conform(&tagged_self_contained(), "msgLeiosBlock", CAPTURED_BLOCK);
    }
}
