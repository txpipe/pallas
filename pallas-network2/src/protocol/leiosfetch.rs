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

use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, decode, encode};

use super::{Bitmaps, EbId, EndorserBlockCbor, Error, TxCbor};

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
}
