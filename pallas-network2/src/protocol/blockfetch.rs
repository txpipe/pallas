//! BlockFetch mini-protocol implementation

use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, data::IanaTag, decode, encode};

use crate::protocol::Error;

use super::Point;

/// Protocol channel number for node-to-node block-fetch
pub const CHANNEL_ID: u16 = 3;

/// Raw bytes of a fetched block body.
pub type Body = Vec<u8>;

/// A range of blocks defined by two points (inclusive).
pub type Range = (Point, Point);

/// A block-fetch mini-protocol message.
#[derive(Debug, Clone)]
pub enum Message {
    /// Client requests a range of blocks.
    RequestRange(Range),
    /// Client signals it is done fetching blocks.
    ClientDone,
    /// Server signals the start of a batch of blocks.
    StartBatch,
    /// Server signals that no blocks are available for the requested range.
    NoBlocks,
    /// Server sends a single block body.
    Block(Body),
    /// Server signals the end of a batch.
    BatchDone,
}

/// State machine for the block-fetch mini-protocol.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum State {
    /// Client has agency; can request a range or signal done.
    #[default]
    Idle,
    /// A range has been requested; waiting for server to start or refuse.
    Busy(Range),
    /// Server is streaming blocks; contains the last received block if any.
    Streaming(Option<Body>),
    /// The protocol has terminated.
    Done,
}

impl State {
    /// Applies a message to the current state, returning the new state.
    pub fn apply(&self, msg: &Message) -> Result<Self, Error> {
        match self {
            Self::Idle => match msg {
                Message::RequestRange(range) => Ok(Self::Busy(range.clone())),
                Message::ClientDone => Ok(Self::Done),
                _ => Err(Error::InvalidOutbound),
            },
            Self::Busy(_) => match msg {
                Message::NoBlocks => Ok(Self::Idle),
                Message::StartBatch => Ok(Self::Streaming(None)),
                _ => Err(Error::InvalidInbound),
            },
            Self::Streaming(..) => match msg {
                Message::Block(body) => Ok(Self::Streaming(Some(body.clone()))),
                Message::BatchDone => Ok(Self::Idle),
                _ => Err(Error::InvalidInbound),
            },
            Self::Done => Err(Error::InvalidOutbound),
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
            Message::RequestRange(range) => {
                e.array(3)?.u16(0)?;
                e.encode(&range.0)?;
                e.encode(&range.1)?;
                Ok(())
            }
            Message::ClientDone => {
                e.array(1)?.u16(1)?;
                Ok(())
            }
            Message::StartBatch => {
                e.array(1)?.u16(2)?;
                Ok(())
            }
            Message::NoBlocks => {
                e.array(1)?.u16(3)?;
                Ok(())
            }
            Message::Block(body) => {
                e.array(2)?.u16(4)?;
                e.tag(IanaTag::Cbor)?;
                e.bytes(body)?;
                Ok(())
            }
            Message::BatchDone => {
                e.array(1)?.u16(5)?;
                Ok(())
            }
        }
    }
}

impl<'b> Decode<'b, ()> for Message {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => {
                let point1 = d.decode()?;
                let point2 = d.decode()?;
                Ok(Message::RequestRange((point1, point2)))
            }
            1 => Ok(Message::ClientDone),
            2 => Ok(Message::StartBatch),
            3 => Ok(Message::NoBlocks),
            4 => {
                d.tag()?;
                let body = d.bytes()?;
                Ok(Message::Block(Vec::from(body)))
            }
            5 => Ok(Message::BatchDone),
            _ => Err(decode::Error::message(
                "unknown variant for blockfetch message",
            )),
        }
    }
}
