use std::fmt::Debug;

use log::{debug, log_enabled, trace, warn};
use minicbor::encode;
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

pub type WrappedHeader = Vec<u8>;

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

#[derive(Debug)]
pub enum Message {
    RequestNext,
    AwaitReply,
    RollForward(WrappedHeader, Tip),
    RollBackward(Point, Tip),
    FindIntersect(Vec<Point>),
    IntersectFound(Point, Tip),
    IntersectNotFound(Tip),
    Done,
}

impl EncodePayload for Message {
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
                e.bytes(&header)?;
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

impl DecodePayload for Message {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => Ok(Message::RequestNext),
            1 => Ok(Message::AwaitReply),
            2 => {
                warn!("{:?}", d.array()?);
                warn!("{:?}", d.u8()?);
                warn!("{:?}", d.tag()?);
                let header = Vec::from(d.bytes()?);
                let tip = Tip::decode_payload(d)?;
                Ok(Message::RollForward(header, tip))
            }
            3 => {
                let point = Point::decode_payload(d)?;
                let tip = Tip::decode_payload(d)?;
                Ok(Message::RollBackward(point, tip))
            }
            4 => {
                let points_len = d
                    .array()?
                    .ok_or(MachineError::UnexpectedCbor("unbounded points array"))?;
                let mut points = Vec::with_capacity(points_len as usize);
                for i in 0..(points_len - 1) {
                    points[i as usize] = Point::decode_payload(d)?;
                }
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
pub struct Consumer {
    pub state: State,
    pub known_points: Vec<Point>,
    pub cursor: Option<Point>,
    pub tip: Option<Tip>,
}

impl Consumer {
    pub fn initial(known_points: Vec<Point>) -> Self {
        Self {
            state: State::Idle,
            cursor: None,
            tip: None,
            known_points,
        }
    }

    fn send_find_intersect(self, tx: &impl MachineOutput) -> Transition<Self> {
        let msg = Message::FindIntersect(self.known_points.clone());

        tx.send_msg(&msg)?;

        Ok(Self {
            state: State::Intersect,
            ..self
        })
    }

    fn send_request_next(self, tx: &impl MachineOutput) -> Transition<Self> {
        let msg = Message::RequestNext;

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

    fn on_roll_forward(self, header: Vec<u8>, tip: Tip) -> Transition<Self> {
        debug!("rolling forward: {:?}", header);

        if log_enabled!(log::Level::Trace) {
            trace!("header: {}", hex::encode(&header));
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

impl Agent for Consumer {
    type Message = Message;

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
