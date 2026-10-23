//! LeiosNotify mini-protocol implementation.
//!
//! Server-push protocol by which a peer announces new Endorser Blocks (EBs),
//! offers their bodies and transactions for eager fetching over
//! [`super::leiosfetch`], and diffuses full votes inline. The client repeatedly
//! asks for the next notification; the server replies with exactly one
//! announcement/offer and returns to idle.
//!
//! Wire format and state machine follow the authoritative `leios-notify` CDDL of
//! cardano-blueprint (protocol id 18), which is the network spec of record while
//! CIP-0164's network chapter stabilises.

use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, decode, encode};

use super::{AnyCbor, EbId, Error};

/// Protocol channel number for node-to-node leios-notify.
pub const CHANNEL_ID: u16 = 18;

/// Raw CBOR of a single Leios vote (persistent or non-persistent).
pub type VoteCbor = AnyCbor;

/// A leios-notify mini-protocol message.
#[derive(Debug, Clone)]
pub enum Message {
    /// Server closes the protocol after the client quits.
    Done,
    /// Client ends the protocol.
    Quit,
    /// Server answers a request with no notification.
    Canceled,
    /// Client requests the next notification.
    RequestNext,
    /// Server announces an EB via the raw CBOR of the announcing RB header.
    BlockAnnouncement(AnyCbor),
    /// Server offers an EB body it can deliver, with its size in bytes.
    BlockOffer(EbId, u32),
    /// Server offers the transactions of an EB it can deliver.
    BlockTxsOffer(EbId),
    /// Server diffuses full votes inline: `[7, [vote, ...]]`.
    Votes(Vec<VoteCbor>),
}

/// A notification delivered by the server, retained in the idle state until the
/// consumer drains it (mirrors the chain-sync `Data` idiom).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notification {
    /// An EB announcement (raw CBOR of the announcing RB header).
    BlockAnnouncement(AnyCbor),
    /// An EB body is available, with its size in bytes.
    BlockOffer(EbId, u32),
    /// The transactions of an EB are available.
    BlockTxsOffer(EbId),
    /// Full votes diffused inline by the server.
    Votes(Vec<VoteCbor>),
}

/// State machine for the leios-notify mini-protocol.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Client has agency; can request the next notification or finish. Carries
    /// any not-yet-drained notification delivered by the server.
    Idle(Option<Notification>),
    /// Server has agency; will deliver one announcement/offer.
    Busy,
    /// Server has agency after the client quit and will close the protocol.
    Quit,
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
                Message::RequestNext => Ok(State::Busy),
                Message::Quit => Ok(State::Quit),
                _ => Err(Error::InvalidOutbound),
            },
            State::Busy => match msg {
                Message::BlockAnnouncement(h) => Ok(State::Idle(Some(
                    Notification::BlockAnnouncement(h.clone()),
                ))),
                Message::BlockOffer(p, s) => {
                    Ok(State::Idle(Some(Notification::BlockOffer(p.clone(), *s))))
                }
                Message::BlockTxsOffer(p) => {
                    Ok(State::Idle(Some(Notification::BlockTxsOffer(p.clone()))))
                }
                Message::Votes(v) => Ok(State::Idle(Some(Notification::Votes(v.clone())))),
                Message::Canceled => Ok(State::Idle(None)),
                _ => Err(Error::InvalidInbound),
            },
            State::Quit => match msg {
                Message::Done => Ok(State::Done),
                _ => Err(Error::InvalidInbound),
            },
            State::Done => Err(Error::InvalidOutbound),
        }
    }

    /// Takes any pending notification, leaving the state idle. Returns `None` if
    /// there is nothing to drain or the protocol is not idle.
    pub fn drain(&mut self) -> Option<Notification> {
        match self {
            State::Idle(n) => n.take(),
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
            Message::Quit => {
                e.array(1)?.u16(1)?;
            }
            Message::Canceled => {
                e.array(1)?.u16(2)?;
            }
            Message::RequestNext => {
                e.array(1)?.u16(3)?;
            }
            Message::BlockAnnouncement(header) => {
                e.array(2)?.u16(4)?;
                e.encode(header)?;
            }
            Message::BlockOffer(point, size) => {
                e.array(3)?.u16(5)?;
                e.encode(point)?;
                e.u32(*size)?;
            }
            Message::BlockTxsOffer(point) => {
                e.array(2)?.u16(6)?;
                e.encode(point)?;
            }
            Message::Votes(votes) => {
                e.array(2)?.u16(7)?;
                e.encode(votes)?;
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
            1 => Ok(Message::Quit),
            2 => Ok(Message::Canceled),
            3 => Ok(Message::RequestNext),
            4 => Ok(Message::BlockAnnouncement(d.decode()?)),
            5 => {
                let point = d.decode()?;
                let size = d.u32()?;
                Ok(Message::BlockOffer(point, size))
            }
            6 => Ok(Message::BlockTxsOffer(d.decode()?)),
            7 => Ok(Message::Votes(d.decode()?)),
            _ => Err(decode::Error::message(
                "unknown variant for leiosnotify message",
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

    fn point() -> EbId {
        Point::Specific(42, vec![0xAB; 32])
    }

    fn roundtrip(msg: &Message) -> Message {
        let bytes = minicbor::to_vec(msg).unwrap();
        minicbor::decode(&bytes).unwrap()
    }

    #[test]
    fn message_roundtrips() {
        let cases = vec![
            Message::RequestNext,
            Message::BlockAnnouncement(AnyCbor::from_raw_bytes(
                minicbor::to_vec([1u8, 2, 3]).unwrap(),
            )),
            Message::BlockOffer(point(), 12345),
            Message::BlockTxsOffer(point()),
            Message::Votes(vec![
                AnyCbor::from_raw_bytes(minicbor::to_vec([9u8, 8, 7, 6]).unwrap()),
                AnyCbor::from_raw_bytes(minicbor::to_vec([5u8, 4, 3, 2]).unwrap()),
            ]),
            Message::Done,
            Message::Quit,
            Message::Canceled,
        ];

        for msg in &cases {
            let back = roundtrip(msg);
            // compare via re-encode since Message is not PartialEq
            assert_eq!(
                minicbor::to_vec(&back).unwrap(),
                minicbor::to_vec(msg).unwrap()
            );
        }
    }

    fn decode_captured(bytes: &[u8]) -> Message {
        let msg: Message = minicbor::decode(bytes).expect("captured message decodes");
        assert_eq!(
            minicbor::to_vec(&msg).unwrap(),
            bytes,
            "re-encodes byte equal"
        );
        msg
    }

    fn captured_point() -> EbId {
        Point::Specific(
            353,
            hex::decode("a085f5ee18bce08a1a9393a54c8bcb6e237ab5d1270681cd4b36648b1d099b8b")
                .unwrap(),
        )
    }

    #[test]
    fn captured_messages_decode_by_their_numbers() {
        let request = decode_captured(include_bytes!(
            "../../../test_data/leios-rc-2026w42/notify-request-next.cbor"
        ));
        assert!(matches!(request, Message::RequestNext), "{request:?}");

        let announcement =
            include_bytes!("../../../test_data/leios-rc-2026w42/notify-announcement.cbor");
        match decode_captured(announcement) {
            Message::BlockAnnouncement(header) => {
                assert_eq!(header.raw_bytes(), &announcement[2..])
            }
            other => panic!("{other:?}"),
        }

        match decode_captured(include_bytes!(
            "../../../test_data/leios-rc-2026w42/notify-offer.cbor"
        )) {
            Message::BlockOffer(point, size) => {
                assert_eq!(point, captured_point());
                assert_eq!(size, 3665);
            }
            other => panic!("{other:?}"),
        }

        match decode_captured(include_bytes!(
            "../../../test_data/leios-rc-2026w42/notify-txs-offer.cbor"
        )) {
            Message::BlockTxsOffer(point) => assert_eq!(point, captured_point()),
            other => panic!("{other:?}"),
        }

        match decode_captured(include_bytes!(
            "../../../test_data/leios-rc-2026w42/notify-votes.cbor"
        )) {
            Message::Votes(votes) => assert_eq!(votes.len(), 1),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn state_transitions_and_drain() {
        let s = State::default();
        assert_eq!(s, State::Idle(None));

        let busy = s.apply(&Message::RequestNext).unwrap();
        assert_eq!(busy, State::Busy);

        let mut idle = busy.apply(&Message::BlockTxsOffer(point())).unwrap();
        assert_eq!(
            idle,
            State::Idle(Some(Notification::BlockTxsOffer(point())))
        );

        // draining yields the notification once and leaves the state idle/empty
        assert_eq!(idle.drain(), Some(Notification::BlockTxsOffer(point())));
        assert_eq!(idle, State::Idle(None));
        assert_eq!(idle.drain(), None);

        let quit = idle.apply(&Message::Quit).unwrap();
        assert_eq!(quit, State::Quit);

        let done = quit.apply(&Message::Done).unwrap();
        assert_eq!(done, State::Done);
    }

    #[test]
    fn termination_messages_take_their_numbers_and_states() {
        assert_eq!(minicbor::to_vec(Message::Done).unwrap(), [0x81, 0x00]);
        assert_eq!(minicbor::to_vec(Message::Quit).unwrap(), [0x81, 0x01]);
        assert_eq!(minicbor::to_vec(Message::Canceled).unwrap(), [0x81, 0x02]);

        let pending = State::Idle(Some(Notification::BlockTxsOffer(point())));
        assert_eq!(pending.apply(&Message::Quit).unwrap(), State::Quit);
        assert_eq!(
            State::Busy.apply(&Message::Canceled).unwrap(),
            State::Idle(None)
        );

        assert!(matches!(
            State::Idle(None).apply(&Message::Done),
            Err(Error::InvalidOutbound)
        ));
        assert!(matches!(
            State::Idle(None).apply(&Message::Canceled),
            Err(Error::InvalidOutbound)
        ));
        assert!(matches!(
            State::Busy.apply(&Message::Done),
            Err(Error::InvalidInbound)
        ));
        assert!(matches!(
            State::Quit.apply(&Message::Canceled),
            Err(Error::InvalidInbound)
        ));
        assert!(matches!(
            State::Quit.apply(&Message::BlockTxsOffer(point())),
            Err(Error::InvalidInbound)
        ));
    }

    #[test]
    fn illegal_transitions_error() {
        // offer while idle is invalid
        assert!(matches!(
            State::Idle(None).apply(&Message::BlockTxsOffer(point())),
            Err(Error::InvalidOutbound)
        ));
        // request while busy is invalid
        assert!(matches!(
            State::Busy.apply(&Message::RequestNext),
            Err(Error::InvalidInbound)
        ));
    }

    // --- CBOR-vs-CDDL conformance (run with `--features blueprint`) ---
    //
    // Each `conforms!` below emits one `#[test]` that encodes a sample message
    // with our `Encode` impl and validates the bytes against the vendored
    // cardano-blueprint leios-notify CDDL (via the shared `cddl` helper),
    // so a spec change (tag, arity, the inline vote shape) fails the matching
    // test.

    /// Turns the vendored leios-notify CDDL into a schema cddl-rs can parse, with
    /// the imported announcement header opaque.
    #[cfg(feature = "blueprint")]
    fn self_contained() -> String {
        let body = cddl::preprocess(include_str!(
            "../../../cardano-blueprint/src/network/node-to-node/leios-notify/messages.cddl"
        ))
        .replace("header.header", "any");
        format!("{body}\n{}", cddl::BASE_PRELUDE)
    }

    /// A conformant `vote = [announcing_rb_hash, voter_id, vote_signature(.size 48)]`.
    #[cfg(feature = "blueprint")]
    fn vote() -> VoteCbor {
        let mut buf = Vec::new();
        Encoder::new(&mut buf)
            .array(3)
            .unwrap()
            .bytes(&[0xAB; 32])
            .unwrap()
            .u16(3)
            .unwrap()
            .bytes(&[0xEE; 48])
            .unwrap();
        AnyCbor::from_raw_bytes(buf)
    }

    conforms!(
        request_next_conforms,
        self_contained,
        "msgLeiosNotificationRequestNext",
        Message::RequestNext
    );
    conforms!(
        block_announcement_conforms,
        self_contained,
        "msgLeiosBlockAnnouncement",
        Message::BlockAnnouncement(AnyCbor::from_raw_bytes(
            minicbor::to_vec([1u8, 2, 3]).unwrap()
        ))
    );
    conforms!(
        block_offer_conforms,
        self_contained,
        "msgLeiosBlockOffer",
        Message::BlockOffer(point(), 1234)
    );
    conforms!(
        block_txs_offer_conforms,
        self_contained,
        "msgLeiosBlockTxsOffer",
        Message::BlockTxsOffer(point())
    );
    conforms!(
        votes_conforms,
        self_contained,
        "msgLeiosVotes",
        Message::Votes(vec![vote()])
    );
    conforms!(done_conforms, self_contained, "msgDone", Message::Done);
    conforms!(quit_conforms, self_contained, "msgQuit", Message::Quit);
    conforms!(
        canceled_conforms,
        self_contained,
        "msgCanceled",
        Message::Canceled
    );
}
