use crate::machines::{Agent, MachineOutput, Transition};

use crate::common::Point;

use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

#[derive(Debug, PartialEq, Clone)]
pub enum State {
    Idle,
    Busy,
    Streaming,
    Done,
}

#[derive(Debug)]
pub enum Message {
    RequestRange { range: (Point, Point) },
    ClientDone,
    StartBatch,
    NoBlocks,
    Block { body: Vec<u8> },
    BatchDone,
}

impl Encode for Message {
    fn encode<W: encode::Write>(&self, e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::RequestRange { range } => {
                e.array(3)?.u16(0)?;
                range.0.encode(e)?;
                range.1.encode(e)?;
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
            Message::Block { body } => {
                e.array(2)?.u16(4)?;
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

impl<'b> Decode<'b> for Message {
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => {
                let point1 = Point::decode(d)?;
                let point2 = Point::decode(d)?;
                Ok(Message::RequestRange {
                    range: (point1, point2),
                })
            }
            1 => Ok(Message::ClientDone),
            2 => Ok(Message::StartBatch),
            3 => Ok(Message::NoBlocks),
            4 => {
                d.tag()?;
                let body = d.bytes()?;
                Ok(Message::Block {
                    body: Vec::from(body),
                })
            }
            5 => Ok(Message::BatchDone),
            _ => Err(decode::Error::message(
                "unknown variant for blockfetch message",
            )),
        }
    }
}

pub trait Observer {
    fn on_block_received(&self, body: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("block received, sice: {}", body.len());
        Ok(())
    }

    fn on_block_range_requested(
        &self,
        range: &(Point, Point),
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!(
            "block range requested, from: {:?}, to: {:?}",
            range.0,
            range.1
        );
        Ok(())
    }
}

#[derive(Debug)]
pub struct NoopObserver {}

impl Observer for NoopObserver {}

#[derive(Debug)]
pub struct BatchClient<O>
where
    O: Observer,
{
    pub state: State,
    pub range: (Point, Point),
    pub observer: O,
}

impl<O> BatchClient<O>
where
    O: Observer,
{
    pub fn initial(range: (Point, Point), observer: O) -> Self {
        Self {
            state: State::Idle,
            range,
            observer,
        }
    }

    fn send_request_range(self, tx: &impl MachineOutput) -> Transition<Self> {
        let msg = Message::RequestRange {
            range: self.range.clone(),
        };

        tx.send_msg(&msg)?;

        self.observer.on_block_range_requested(&self.range)?;

        Ok(Self {
            state: State::Busy,
            ..self
        })
    }

    fn on_block(self, body: Vec<u8>) -> Transition<Self> {
        log::debug!("received block body, size {}", body.len());

        self.observer.on_block_received(body)?;

        Ok(self)
    }
}

impl<O> Agent for BatchClient<O>
where
    O: Observer,
{
    type Message = Message;

    fn is_done(&self) -> bool {
        self.state == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state {
            State::Idle => true,
            State::Busy => false,
            State::Streaming => false,
            State::Done => false,
        }
    }

    fn send_next(self, tx: &impl MachineOutput) -> Transition<Self> {
        match self.state {
            State::Idle => self.send_request_range(tx),
            _ => panic!("I don't have agency, don't know what to do"),
        }
    }

    fn receive_next(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::Busy, Message::StartBatch) => Ok(Self {
                state: State::Streaming,
                ..self
            }),
            (State::Busy, Message::NoBlocks) => Ok(Self {
                state: State::Done,
                ..self
            }),
            (State::Streaming, Message::Block { body }) => self.on_block(body),
            (State::Streaming, Message::BatchDone) => Ok(Self {
                state: State::Done,
                ..self
            }),
            _ => panic!("I have agency, I don't expect messages"),
        }
    }
}

#[derive(Debug)]
pub struct OnDemandClient<I, O>
where
    I: Iterator<Item = Point>,
    O: Observer,
{
    pub state: State,
    pub requests: I,
    pub observer: O,
}

impl<I, O> OnDemandClient<I, O>
where
    I: Iterator<Item = Point>,
    O: Observer,
{
    pub fn initial(requests: I, observer: O) -> Self {
        Self {
            state: State::Idle,
            requests,
            observer,
        }
    }

    fn send_request_range(self, tx: &impl MachineOutput, point: Point) -> Transition<Self> {
        log::debug!("requesting block {:?}", point);

        let msg = Message::RequestRange {
            range: (point.clone(), point),
        };

        tx.send_msg(&msg)?;

        Ok(Self {
            state: State::Busy,
            ..self
        })
    }

    fn send_done(self, tx: &impl MachineOutput) -> Transition<Self> {
        log::debug!("notifying done");

        let msg = Message::ClientDone;

        //tx.send_msg(&msg)?;

        Ok(Self {
            state: State::Done,
            ..self
        })
    }

    fn wait_for_request_and_send(mut self, tx: &impl MachineOutput) -> Transition<Self> {
        let point = self.requests.next();

        match point {
            Some(x) => self.send_request_range(tx, x),
            None => self.send_done(tx),
        }
    }

    fn on_block(self, body: Vec<u8>) -> Transition<Self> {
        log::debug!("received block body, size {}", body.len());

        self.observer.on_block_received(body)?;

        Ok(self)
    }
}

impl<I, O> Agent for OnDemandClient<I, O>
where
    I: Iterator<Item = Point>,
    O: Observer,
{
    type Message = Message;

    fn is_done(&self) -> bool {
        self.state == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state {
            State::Idle => true,
            State::Busy => false,
            State::Streaming => false,
            State::Done => false,
        }
    }

    fn send_next(self, tx: &impl MachineOutput) -> Transition<Self> {
        match self.state {
            State::Idle => self.wait_for_request_and_send(tx),
            _ => panic!("I don't have agency, don't know what to do"),
        }
    }

    fn receive_next(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::Busy, Message::StartBatch) => Ok(Self {
                state: State::Streaming,
                ..self
            }),
            (State::Busy, Message::NoBlocks) => Ok(Self {
                state: State::Idle,
                ..self
            }),
            (State::Streaming, Message::Block { body }) => self.on_block(body),
            (State::Streaming, Message::BatchDone) => Ok(Self {
                state: State::Idle,
                ..self
            }),
            _ => panic!("I have agency, I don't expect messages"),
        }
    }
}
