use std::fmt::Debug;

use log::{debug, log_enabled, trace};

use minicbor::data::Tag;
use pallas_machines::{
    Agent, DecodePayload, EncodePayload, MachineError, MachineOutput, PayloadDecoder,
    PayloadEncoder, Transition,
};

#[derive(Clone)]
pub struct Point(pub u64, pub Vec<u8>);

impl Debug for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Point")
            .field(&self.0)
            .field(&hex::encode(&self.1))
            .finish()
    }
}

impl EncodePayload for Point {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        e.array(2)?.u64(self.0)?.bytes(&self.1)?;
        Ok(())
    }
}

impl DecodePayload for Point {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let slot = d.u64()?;
        let hash = d.bytes()?;

        Ok(Point(slot, Vec::from(hash)))
    }
}

#[derive(Debug)]
pub struct WrappedHeader(u64, Vec<u8>);

impl EncodePayload for WrappedHeader {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        e.array(2)?;
        e.u64(self.0)?;
        e.tag(Tag::Cbor)?;
        e.bytes(&self.1)?;

        Ok(())
    }
}

impl DecodePayload for WrappedHeader {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let unknown = d.u64()?; // WTF is this value?
        d.tag()?;
        let bytes = Vec::from(d.bytes()?);

        Ok(WrappedHeader(unknown, bytes))
    }
}

#[derive(Debug)]
pub struct BlockBody(Vec<u8>);

impl EncodePayload for BlockBody {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }
}

impl DecodePayload for BlockBody {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.tag()?;
        let bytes = Vec::from(d.bytes()?);

        Ok(BlockBody(bytes))
    }
}

#[derive(Debug)]
pub struct Tip(Point, u64);

impl EncodePayload for Tip {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        e.array(2)?;
        self.0.encode_payload(e)?;
        e.u64(self.1)?;

        Ok(())
    }
}

impl DecodePayload for Tip {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let point = Point::decode_payload(d)?;
        let block_num = d.u64()?;

        Ok(Tip(point, block_num))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum State {
    Idle,
    CanAwait,
    MustReply,
    Intersect,
    Done,
}

/// A generic chain-sync message for either header or block content
#[derive(Debug)]
pub enum Message<C>
where
    C: EncodePayload + DecodePayload,
{
    RequestNext,
    AwaitReply,
    RollForward(C, Tip),
    RollBackward(Point, Tip),
    FindIntersect(Vec<Point>),
    IntersectFound(Point, Tip),
    IntersectNotFound(Tip),
    Done,
}

impl<C> EncodePayload for Message<C>
where
    C: EncodePayload + DecodePayload,
{
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Message::RequestNext => {
                e.array(1)?.u16(0)?;
                Ok(())
            }
            Message::AwaitReply => {
                e.array(1)?.u16(1)?;
                Ok(())
            }
            Message::RollForward(header, tip) => {
                e.array(3)?.u16(2)?;
                header.encode_payload(e)?;
                tip.encode_payload(e)?;
                Ok(())
            }
            Message::RollBackward(point, tip) => {
                e.array(3)?.u16(3)?;
                point.encode_payload(e)?;
                tip.encode_payload(e)?;
                Ok(())
            }
            Message::FindIntersect(points) => {
                e.array(2)?.u16(4)?;
                e.array(points.len() as u64)?;
                for point in points.iter() {
                    point.encode_payload(e)?;
                }
                Ok(())
            }
            Message::IntersectFound(point, tip) => {
                e.array(3)?.u16(5)?;
                point.encode_payload(e)?;
                tip.encode_payload(e)?;
                Ok(())
            }
            Message::IntersectNotFound(tip) => {
                e.array(1)?.u16(6)?;
                tip.encode_payload(e)?;
                Ok(())
            }
            Message::Done => {
                e.array(1)?.u16(7)?;
                Ok(())
            }
        }
    }
}

impl<C> DecodePayload for Message<C>
where
    C: EncodePayload + DecodePayload,
{
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => Ok(Message::RequestNext),
            1 => Ok(Message::AwaitReply),
            2 => {
                let content = C::decode_payload(d)?;
                let tip = Tip::decode_payload(d)?;
                Ok(Message::RollForward(content, tip))
            }
            3 => {
                let point = Point::decode_payload(d)?;
                let tip = Tip::decode_payload(d)?;
                Ok(Message::RollBackward(point, tip))
            }
            4 => {
                let points = Vec::<Point>::decode_payload(d)?;
                Ok(Message::FindIntersect(points))
            }
            5 => {
                let point = Point::decode_payload(d)?;
                let tip = Tip::decode_payload(d)?;
                Ok(Message::IntersectFound(point, tip))
            }
            6 => {
                let tip = Tip::decode_payload(d)?;
                Ok(Message::IntersectNotFound(tip))
            }
            7 => Ok(Message::Done),
            x => Err(Box::new(MachineError::BadLabel(x))),
        }
    }
}

#[derive(Debug)]
pub struct Consumer<C> {
    pub state: State,
    pub known_points: Vec<Point>,
    pub cursor: Option<Point>,
    pub tip: Option<Tip>,

    // as recommended here: https://doc.rust-lang.org/error-index.html#E0207
    _phantom: Option<C>,
}

impl<C> Consumer<C>
where
    C: EncodePayload + DecodePayload + Debug,
{
    pub fn initial(known_points: Vec<Point>) -> Self {
        Self {
            state: State::Idle,
            cursor: None,
            tip: None,
            known_points,
            _phantom: None,
        }
    }

    fn send_find_intersect(self, tx: &impl MachineOutput) -> Transition<Self> {
        let msg = Message::<C>::FindIntersect(self.known_points.clone());

        tx.send_msg(&msg)?;

        Ok(Self {
            state: State::Intersect,
            ..self
        })
    }

    fn send_request_next(self, tx: &impl MachineOutput) -> Transition<Self> {
        let msg = Message::<C>::RequestNext;

        tx.send_msg(&msg)?;

        Ok(Self {
            state: State::CanAwait,
            ..self
        })
    }

    fn on_intersect_found(self, point: Point, tip: Tip) -> Transition<Self> {
        debug!("intersect found: {:?} (tip: {:?})", point, tip);

        Ok(Self {
            tip: Some(tip),
            cursor: Some(point),
            state: State::Idle,
            ..self
        })
    }

    fn on_intersect_not_found(self, tip: Tip) -> Transition<Self> {
        debug!("intersect not found (tip: {:?})", tip);

        Ok(Self {
            tip: Some(tip),
            cursor: None,
            state: State::Idle,
            ..self
        })
    }

    fn on_roll_forward(self, content: C, tip: Tip) -> Transition<Self> {
        debug!("rolling forward");

        if log_enabled!(log::Level::Trace) {
            trace!("header: {:?}", content);
        }

        Ok(Self {
            tip: Some(tip),
            state: State::Idle,
            ..self
        })
    }

    fn on_roll_backward(self, point: Point, tip: Tip) -> Transition<Self> {
        debug!("rolling backward to point: {:?}", point);

        Ok(Self {
            tip: Some(tip),
            cursor: Some(point),
            state: State::Idle,
            ..self
        })
    }
}

impl<C> Agent for Consumer<C>
where
    C: EncodePayload + DecodePayload + Debug,
{
    type Message = Message<C>;

    fn is_done(&self) -> bool {
        self.state == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state {
            State::Idle => true,
            State::CanAwait => false,
            State::MustReply => false,
            State::Intersect => false,
            State::Done => false,
        }
    }

    fn send_next(self, tx: &impl MachineOutput) -> Transition<Self> {
        match self.state {
            State::Idle => match self.cursor {
                Some(_) => self.send_request_next(tx),
                None => self.send_find_intersect(tx),
            },
            _ => panic!("I don't have agency, don't know what to do"),
        }
    }

    fn receive_next(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::CanAwait, Message::RollForward(header, tip)) => {
                self.on_roll_forward(header, tip)
            }
            (State::CanAwait, Message::RollBackward(point, tip)) => {
                self.on_roll_backward(point, tip)
            }
            (State::MustReply, Message::RollForward(header, tip)) => {
                self.on_roll_forward(header, tip)
            }
            (State::MustReply, Message::RollBackward(point, tip)) => {
                self.on_roll_backward(point, tip)
            }
            (State::Intersect, Message::IntersectFound(point, tip)) => {
                self.on_intersect_found(point, tip)
            }
            (State::Intersect, Message::IntersectNotFound(tip)) => self.on_intersect_not_found(tip),
            _ => Err(Box::new(MachineError::InvalidMsgForState)),
        }
    }
}

pub type NodeConsumer = Consumer<WrappedHeader>;

pub type ClientConsumer = Consumer<BlockBody>;
