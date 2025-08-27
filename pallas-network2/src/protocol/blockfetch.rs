//! BlockFetch mini-protocol implementation

use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, data::IanaTag, decode, encode};

use crate::protocol::Error;

use super::Point;

/// Protocol channel number for node-to-node block-fetch
pub const CHANNEL_ID: u16 = 3;

pub type Body = Vec<u8>;

pub type Range = (Point, Point);

#[derive(Debug, Clone)]
pub enum Message {
    RequestRange(Range),
    ClientDone,
    StartBatch,
    NoBlocks,
    Block(Body),
    BatchDone,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    Busy(Range),
    Streaming(Option<Body>),
    Done,
}

impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

impl State {
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
