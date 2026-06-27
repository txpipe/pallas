//! LeiosFetch mini-protocol implementation.
//!
//! Client-pull protocol for fetching Endorser Block (EB) bodies, their
//! transactions (selected by a compact bitmap) and votes, discovered via
//! [`super::leiosnotify`]. The client issues one request and the server replies
//! with the matching response, returning to idle.
//!
//! Wire format and state machine follow the `leios-fetch` protocol of the Go
//! reference implementation (protocol id 19); the running prototype is treated
//! as ground truth where it diverges from CIP-0164.

use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, decode, encode};

use super::{Bitmaps, EbId, EndorserBlockCbor, Error, TxCbor, VoteCbor, VoteId};

/// Protocol channel number for node-to-node leios-fetch.
pub const CHANNEL_ID: u16 = 19;

/// A leios-fetch mini-protocol message.
#[derive(Debug, Clone)]
pub enum Message {
    /// Client requests a complete EB body.
    BlockRequest(EbId),
    /// Server delivers an EB body (raw CBOR).
    Block(EndorserBlockCbor),
    /// Client requests a subset of an EB's transactions, selected by bitmap.
    BlockTxsRequest(EbId, Bitmaps),
    /// Server delivers transactions for an EB.
    ///
    /// Two wire shapes are accepted for prototype interop: the 2-element dingo
    /// form `[txs]` (point/bitmaps absent) and the 4-element prototype form
    /// `[point, bitmaps, txs]` echoing the request.
    BlockTxs {
        /// Echoed EB point (prototype form only).
        point: Option<EbId>,
        /// Echoed bitmap selector (prototype form only).
        bitmaps: Option<Bitmaps>,
        /// The requested transactions, as raw CBOR.
        txs: Vec<TxCbor>,
    },
    /// Client requests specific votes by id.
    VotesRequest(Vec<VoteId>),
    /// Server delivers full votes (raw CBOR).
    Votes(Vec<VoteCbor>),
    /// Client requests a range of EBs (catch-up). Not yet live in the prototype;
    /// encoded/decoded for forward compatibility only.
    BlockRangeRequest(EbId, EbId),
    /// Server delivers the final EB+txs of a range, returning to idle.
    ///
    /// Note: the message tag is `7` and [`Message::NextBlockAndTxsInRange`] is
    /// `8`, matching the Go reference implementation — the *opposite* of the
    /// ordering in CIP-0164. Unverified against a live node (range fetch is not
    /// yet implemented by the prototype).
    LastBlockAndTxsInRange(EndorserBlockCbor, Vec<TxCbor>),
    /// Server delivers an intermediate EB+txs of a range, staying in range mode.
    NextBlockAndTxsInRange(EndorserBlockCbor, Vec<TxCbor>),
    /// Client terminates the protocol.
    Done,
}

/// A response delivered by the server, retained in the idle state until the
/// consumer drains it (mirrors the chain-sync `Data` idiom).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// An EB body (raw CBOR).
    Block(EndorserBlockCbor),
    /// Transactions delivered for an EB.
    BlockTxs {
        /// Echoed EB point (prototype form only).
        point: Option<EbId>,
        /// Echoed bitmap selector (prototype form only).
        bitmaps: Option<Bitmaps>,
        /// The delivered transactions.
        txs: Vec<TxCbor>,
    },
    /// Full votes (raw CBOR).
    Votes(Vec<VoteCbor>),
}

/// State machine for the leios-fetch mini-protocol.
///
/// The `Awaiting*` states retain the request parameters so a responder can serve
/// them; the `Idle` state retains the delivered response until the consumer
/// drains it.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Client has agency; can issue a request or finish.
    Idle(Option<Response>),
    /// Server has agency; will deliver the requested EB body.
    AwaitingBlock(EbId),
    /// Server has agency; will deliver transactions for the requested EB.
    AwaitingBlockTxs(EbId, Bitmaps),
    /// Server has agency; will deliver the requested votes.
    AwaitingVotes(Vec<VoteId>),
    /// Server has agency; streaming the requested range of EBs.
    AwaitingRange(EbId, EbId),
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
                Message::VotesRequest(ids) => Ok(State::AwaitingVotes(ids.clone())),
                Message::BlockRangeRequest(s, e) => Ok(State::AwaitingRange(s.clone(), e.clone())),
                Message::Done => Ok(State::Done),
                _ => Err(Error::InvalidOutbound),
            },
            State::AwaitingBlock(_) => match msg {
                Message::Block(b) => Ok(State::Idle(Some(Response::Block(b.clone())))),
                _ => Err(Error::InvalidInbound),
            },
            State::AwaitingBlockTxs(..) => match msg {
                Message::BlockTxs {
                    point,
                    bitmaps,
                    txs,
                } => Ok(State::Idle(Some(Response::BlockTxs {
                    point: point.clone(),
                    bitmaps: bitmaps.clone(),
                    txs: txs.clone(),
                }))),
                _ => Err(Error::InvalidInbound),
            },
            State::AwaitingVotes(_) => match msg {
                Message::Votes(v) => Ok(State::Idle(Some(Response::Votes(v.clone())))),
                _ => Err(Error::InvalidInbound),
            },
            State::AwaitingRange(s, e) => match msg {
                // Range items are not surfaced in this first cut; streaming stays
                // in range mode until the final item returns to idle.
                Message::NextBlockAndTxsInRange(..) => {
                    Ok(State::AwaitingRange(s.clone(), e.clone()))
                }
                Message::LastBlockAndTxsInRange(..) => Ok(State::Idle(None)),
                _ => Err(Error::InvalidInbound),
            },
            State::Done => Err(Error::InvalidOutbound),
        }
    }

    /// Takes any pending response, leaving the state idle. Returns `None` if
    /// there is nothing to drain or the protocol is not idle.
    pub fn drain(&mut self) -> Option<Response> {
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
            } => match (point, bitmaps) {
                // prototype form: [3, point, bitmaps, txs]
                (Some(point), Some(bitmaps)) => {
                    e.array(4)?.u16(3)?;
                    e.encode(point)?;
                    e.encode(bitmaps)?;
                    e.encode(txs)?;
                }
                // dingo form: [3, txs]
                _ => {
                    e.array(2)?.u16(3)?;
                    e.encode(txs)?;
                }
            },
            Message::VotesRequest(ids) => {
                e.array(2)?.u16(4)?;
                e.encode(ids)?;
            }
            Message::Votes(votes) => {
                e.array(2)?.u16(5)?;
                e.encode(votes)?;
            }
            Message::BlockRangeRequest(start, end) => {
                e.array(3)?.u16(6)?;
                e.encode(start)?;
                e.encode(end)?;
            }
            Message::LastBlockAndTxsInRange(block, txs) => {
                e.array(3)?.u16(7)?;
                e.encode(block)?;
                e.encode(txs)?;
            }
            Message::NextBlockAndTxsInRange(block, txs) => {
                e.array(3)?.u16(8)?;
                e.encode(block)?;
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
        let len = d.array()?;
        let label = d.u16()?;

        match label {
            0 => Ok(Message::BlockRequest(d.decode()?)),
            1 => Ok(Message::Block(d.decode()?)),
            2 => {
                let point = d.decode()?;
                let bitmaps = d.decode()?;
                Ok(Message::BlockTxsRequest(point, bitmaps))
            }
            3 => match len {
                // dingo form: [3, txs]
                Some(2) => Ok(Message::BlockTxs {
                    point: None,
                    bitmaps: None,
                    txs: d.decode()?,
                }),
                // prototype form: [3, point, bitmaps, txs]
                Some(4) => {
                    let point = d.decode()?;
                    let bitmaps = d.decode()?;
                    let txs = d.decode()?;
                    Ok(Message::BlockTxs {
                        point: Some(point),
                        bitmaps: Some(bitmaps),
                        txs,
                    })
                }
                _ => Err(decode::Error::message(
                    "unexpected array length for leiosfetch BlockTxs",
                )),
            },
            4 => Ok(Message::VotesRequest(d.decode()?)),
            5 => Ok(Message::Votes(d.decode()?)),
            6 => {
                let start = d.decode()?;
                let end = d.decode()?;
                Ok(Message::BlockRangeRequest(start, end))
            }
            7 => {
                let block = d.decode()?;
                let txs = d.decode()?;
                Ok(Message::LastBlockAndTxsInRange(block, txs))
            }
            8 => {
                let block = d.decode()?;
                let txs = d.decode()?;
                Ok(Message::NextBlockAndTxsInRange(block, txs))
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
        roundtrip_eq(&Message::VotesRequest(vec![VoteId {
            slot: 5,
            voter_id: 6,
        }]));
        roundtrip_eq(&Message::Votes(vec![raw([1, 2, 3]), raw([4, 5, 6])]));
        roundtrip_eq(&Message::BlockRangeRequest(point(), point()));
        roundtrip_eq(&Message::LastBlockAndTxsInRange(
            raw([1, 1, 1]),
            vec![raw([2, 2, 2])],
        ));
        roundtrip_eq(&Message::NextBlockAndTxsInRange(
            raw([3, 3, 3]),
            vec![raw([4, 4, 4])],
        ));
        roundtrip_eq(&Message::Done);
    }

    #[test]
    fn block_txs_dingo_form() {
        let msg = Message::BlockTxs {
            point: None,
            bitmaps: None,
            txs: vec![raw([7, 7, 7])],
        };
        let bytes = reencode(&msg);
        // envelope is a 2-element array: [tag=3, txs]
        assert_eq!(bytes[0], 0x82);
        let back: Message = minicbor::decode(&bytes).unwrap();
        assert!(matches!(
            back,
            Message::BlockTxs {
                point: None,
                bitmaps: None,
                ..
            }
        ));
    }

    #[test]
    fn block_txs_prototype_form() {
        let msg = Message::BlockTxs {
            point: Some(point()),
            bitmaps: Some(bitmaps()),
            txs: vec![raw([8, 8, 8])],
        };
        let bytes = reencode(&msg);
        // envelope is a 4-element array
        assert_eq!(bytes[0], 0x84);
        let back: Message = minicbor::decode(&bytes).unwrap();
        assert!(matches!(
            back,
            Message::BlockTxs {
                point: Some(_),
                bitmaps: Some(_),
                ..
            }
        ));
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
        assert_eq!(idle, State::Idle(Some(Response::Block(raw([1, 2, 3])))));
        assert_eq!(idle.drain(), Some(Response::Block(raw([1, 2, 3]))));
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
        assert_eq!(
            State::Idle(None)
                .apply(&Message::VotesRequest(vec![VoteId {
                    slot: 1,
                    voter_id: 2
                }]))
                .unwrap(),
            State::AwaitingVotes(vec![VoteId {
                slot: 1,
                voter_id: 2
            }])
        );
    }

    #[test]
    fn range_streaming_stays_then_returns() {
        let s = State::Idle(None)
            .apply(&Message::BlockRangeRequest(point(), point()))
            .unwrap();
        assert_eq!(s, State::AwaitingRange(point(), point()));

        let still = s
            .apply(&Message::NextBlockAndTxsInRange(raw([1, 1, 1]), vec![]))
            .unwrap();
        assert_eq!(
            still,
            State::AwaitingRange(point(), point()),
            "Next keeps streaming"
        );

        let back = still
            .apply(&Message::LastBlockAndTxsInRange(raw([2, 2, 2]), vec![]))
            .unwrap();
        assert_eq!(back, State::Idle(None), "Last returns to idle");
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
}
