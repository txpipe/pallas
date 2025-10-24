use std::{fmt::Debug, ops::Deref};

use crate::protocol::{Error, Point};

use pallas_codec::minicbor;
use pallas_codec::minicbor::{Decode, Decoder, Encode, Encoder, decode, encode};

/// Protocol channel number for node-to-node chain-sync
pub const CHANNEL_ID: u16 = 2;

/// The tip of a chain, characterized by a point and its block height
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tip(pub Point, pub u64);

pub type IntersectResponse = (Option<Point>, Tip);

/// A generic chain-sync message for either header or block content
#[derive(Debug, Clone)]
pub enum Message<C> {
    RequestNext,
    AwaitReply,
    RollForward(C, Tip),
    RollBackward(Point, Tip),
    FindIntersect(Vec<Point>),
    IntersectFound(Point, Tip),
    IntersectNotFound(Tip),
    Done,
}

#[derive(Debug, Clone)]
pub struct HeaderContent {
    pub variant: u8,
    pub byron_prefix: Option<(u8, u64)>,
    pub cbor: Vec<u8>,
}

#[derive(Debug)]
pub struct BlockContent(pub Vec<u8>);

impl Deref for BlockContent {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<BlockContent> for Vec<u8> {
    fn from(other: BlockContent) -> Self {
        other.0
    }
}

#[derive(Debug)]
pub struct SkippedContent;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Data<C> {
    New,
    Intersection(Point, Tip),
    NoIntersection(Tip),
    Content(C, Tip),
    Rollback(Point, Tip),
    Drained,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State<C>
where
    C: Debug + Clone,
{
    Idle(Data<C>),
    CanAwait,
    MustReply,
    Intersect(Vec<Point>),
    Done,
}

impl<C> State<C>
where
    C: Debug + Clone,
{
    pub fn is_new(&self) -> bool {
        matches!(self, Self::Idle(Data::New))
    }

    pub fn is_drained(&self) -> bool {
        matches!(self, Self::Idle(Data::Drained))
    }

    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle(_))
    }

    pub fn drain(&mut self) -> Option<Data<C>> {
        let Self::Idle(data) = self else {
            return None;
        };

        let out = data.clone();

        *self = Self::Idle(Data::Drained);

        Some(out)
    }
}

impl<C> Default for State<C>
where
    C: Debug + Clone,
{
    fn default() -> Self {
        Self::Idle(Data::New)
    }
}

impl<C> From<Data<C>> for State<C>
where
    C: Debug + Clone,
{
    fn from(state: Data<C>) -> Self {
        State::Idle(state)
    }
}

impl<C> State<C>
where
    C: Debug + Clone,
{
    pub fn apply(&self, msg: &Message<C>) -> Result<Self, Error> {
        match self {
            State::Idle(_) => match msg {
                Message::FindIntersect(x) => Ok(State::Intersect(x.clone())),
                Message::RequestNext => Ok(State::CanAwait),
                Message::Done => Ok(State::Done),
                _ => Err(Error::InvalidInbound),
            },
            State::Intersect(_) => match msg {
                Message::IntersectFound(p, t) => {
                    Ok(Data::Intersection(p.clone(), t.clone()).into())
                }
                Message::IntersectNotFound(tip) => Ok(Data::NoIntersection(tip.clone()).into()),
                _ => Err(Error::InvalidInbound),
            },
            State::CanAwait => match msg {
                Message::RollForward(c, t) => Ok(Data::Content(c.clone(), t.clone()).into()),
                Message::RollBackward(p, t) => Ok(Data::Rollback(p.clone(), t.clone()).into()),
                Message::AwaitReply => Ok(State::MustReply),
                _ => Err(Error::InvalidInbound),
            },
            State::MustReply => match msg {
                Message::RollForward(c, t) => Ok(Data::Content(c.clone(), t.clone()).into()),
                Message::RollBackward(p, t) => Ok(Data::Rollback(p.clone(), t.clone()).into()),
                _ => Err(Error::InvalidInbound),
            },

            State::Done => Err(Error::InvalidInbound),
        }
    }
}

impl minicbor::encode::Encode<()> for Tip {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;
        e.encode(&self.0)?;
        e.u64(self.1)?;

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for Tip {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let point = d.decode()?;
        let block_num = d.u64()?;

        Ok(Tip(point, block_num))
    }
}

impl<C> Encode<()> for Message<C>
where
    C: Encode<()>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::RequestNext => {
                e.array(1)?.u16(0)?;
                Ok(())
            }
            Message::AwaitReply => {
                e.array(1)?.u16(1)?;
                Ok(())
            }
            Message::RollForward(content, tip) => {
                e.array(3)?.u16(2)?;
                e.encode(content)?;
                e.encode(tip)?;
                Ok(())
            }
            Message::RollBackward(point, tip) => {
                e.array(3)?.u16(3)?;
                e.encode(point)?;
                e.encode(tip)?;
                Ok(())
            }
            Message::FindIntersect(points) => {
                e.array(2)?.u16(4)?;
                e.array(points.len() as u64)?;
                for point in points.iter() {
                    e.encode(point)?;
                }
                Ok(())
            }
            Message::IntersectFound(point, tip) => {
                e.array(3)?.u16(5)?;
                e.encode(point)?;
                e.encode(tip)?;
                Ok(())
            }
            Message::IntersectNotFound(tip) => {
                e.array(2)?.u16(6)?;
                e.encode(tip)?;
                Ok(())
            }
            Message::Done => {
                e.array(1)?.u16(7)?;
                Ok(())
            }
        }
    }
}

impl<'b, C> Decode<'b, ()> for Message<C>
where
    C: Decode<'b, ()>,
{
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => Ok(Message::RequestNext),
            1 => Ok(Message::AwaitReply),
            2 => {
                let content = d.decode()?;
                let tip = d.decode()?;
                Ok(Message::RollForward(content, tip))
            }
            3 => {
                let point = d.decode()?;
                let tip = d.decode()?;
                Ok(Message::RollBackward(point, tip))
            }
            4 => {
                let points = d.decode()?;
                Ok(Message::FindIntersect(points))
            }
            5 => {
                let point = d.decode()?;
                let tip = d.decode()?;
                Ok(Message::IntersectFound(point, tip))
            }
            6 => {
                let tip = d.decode()?;
                Ok(Message::IntersectNotFound(tip))
            }
            7 => Ok(Message::Done),
            _ => Err(decode::Error::message(
                "unknown variant for chainsync message",
            )),
        }
    }
}

impl<'b> Decode<'b, ()> for HeaderContent {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let variant = d.u8()?; // era variant

        match variant {
            // byron
            0 => {
                d.array()?;

                // can't find a reference anywhere about the structure of these values, but they
                // seem to provide the Byron-specific variant of the header
                let (a, b): (u8, u64) = d.decode()?;

                d.tag()?;
                let bytes = d.bytes()?;

                Ok(HeaderContent {
                    variant,
                    byron_prefix: Some((a, b)),
                    cbor: Vec::from(bytes),
                })
            }
            // shelley and beyond
            _ => {
                d.tag()?;
                let bytes = d.bytes()?;

                Ok(HeaderContent {
                    variant,
                    byron_prefix: None,
                    cbor: Vec::from(bytes),
                })
            }
        }
    }
}

impl Encode<()> for HeaderContent {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?;
        e.u8(self.variant)?;

        // variant 0 is byron
        if self.variant == 0 {
            e.array(2)?;

            if let Some((a, b)) = self.byron_prefix {
                e.array(2)?;
                e.u8(a)?;
                e.u64(b)?;
            } else {
                return Err(minicbor::encode::Error::message(
                    "header variant 0 but no byron prefix",
                ));
            }

            e.tag(minicbor::data::IanaTag::Cbor)?;
            e.bytes(&self.cbor)?;
        } else {
            e.tag(minicbor::data::IanaTag::Cbor)?;
            e.bytes(&self.cbor)?;
        }

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for BlockContent {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.tag()?;
        let bytes = d.bytes()?;
        Ok(BlockContent(Vec::from(bytes)))
    }
}

impl Encode<()> for BlockContent {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.tag(minicbor::data::IanaTag::Cbor)?;
        e.bytes(&self.0)?;

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for SkippedContent {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.skip()?;
        Ok(SkippedContent)
    }
}

impl Encode<()> for SkippedContent {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.null()?;

        Ok(())
    }
}
