use crate::machines::{Agent, Transition};
use crate::MachineError;

use crate::common::Point;

use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};

#[derive(Debug, PartialEq, Eq, Clone)]
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

impl Encode<()> for Message {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::RequestRange { range } => {
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

impl<'b> Decode<'b, ()> for Message {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => {
                let point1 = d.decode()?;
                let point2 = d.decode()?;
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
    fn on_block_received(&mut self, body: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
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

    fn request_range_msg(&self) -> Message {
        Message::RequestRange {
            range: self.range.clone(),
        }
    }

    fn on_range_requested(self) -> Transition<Self> {
        log::debug!("block range requested");

        Ok(Self {
            state: State::Busy,
            ..self
        })
    }

    fn on_block(mut self, body: Vec<u8>) -> Transition<Self> {
        log::debug!("received block body, size {}", body.len());

        self.observer
            .on_block_received(body)
            .map_err(MachineError::downstream)?;

        Ok(self)
    }

    fn on_batch_done(self) -> Transition<Self> {
        Ok(Self {
            state: State::Done,
            ..self
        })
    }

    fn on_client_done(self) -> Transition<Self> {
        Ok(Self {
            state: State::Done,
            ..self
        })
    }
}

impl<O> Agent for BatchClient<O>
where
    O: Observer,
{
    type Message = Message;
    type State = State;

    fn state(&self) -> &Self::State {
        &self.state
    }

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

    fn build_next(&self) -> Self::Message {
        match self.state {
            State::Idle => self.request_range_msg(),
            _ => panic!("I don't have agency, don't know what to do"),
        }
    }

    fn apply_start(self) -> Transition<Self> {
        Ok(Self {
            state: State::Idle,
            ..self
        })
    }

    fn apply_outbound(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::Idle, Message::RequestRange { .. }) => self.on_range_requested(),
            (State::Idle, Message::ClientDone) => self.on_client_done(),
            _ => panic!("I don't have agency, I don't expect outbound message"),
        }
    }

    fn apply_inbound(self, msg: Self::Message) -> Transition<Self> {
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
            (State::Streaming, Message::BatchDone) => self.on_batch_done(),
            _ => panic!("I have agency, I don't expect messages"),
        }
    }
}

#[derive(Debug)]
pub struct OnDemandClient<I, O>
where
    I: IntoIterator<Item = Point>,
    O: Observer,
{
    pub state: State,
    pub inflight: Option<(Point, Point)>,
    pub next: Option<(Point, Point)>,
    pub requests: I::IntoIter,
    pub observer: O,
}

impl<I, O> OnDemandClient<I, O>
where
    I: IntoIterator<Item = Point>,
    O: Observer,
{
    pub fn initial(requests: I, observer: O) -> Self {
        Self {
            state: State::Idle,
            inflight: None,
            next: None,
            requests: requests.into_iter(),
            observer,
        }
    }

    fn wait_for_request(mut self) -> Transition<Self> {
        log::debug!("waiting for external block request");

        let next = self.requests.next();

        match next {
            Some(x) => Ok(Self {
                state: State::Idle,
                next: Some((x.clone(), x)),
                ..self
            }),
            None => Ok(Self {
                state: State::Done,
                next: None,
                ..self
            }),
        }
    }

    fn on_range_requested(self, range: (Point, Point)) -> Transition<Self> {
        log::debug!("requested block range {:?}", range);

        Ok(Self {
            state: State::Busy,
            inflight: Some(range),
            next: None,
            ..self
        })
    }

    fn on_block(mut self, body: Vec<u8>) -> Transition<Self> {
        log::debug!("received block body, size {}", body.len());

        self.observer
            .on_block_received(body)
            .map_err(MachineError::downstream)?;

        Ok(self)
    }

    fn on_batch_done(self) -> Transition<Self> {
        self.wait_for_request()
    }

    fn on_client_done(self) -> Transition<Self> {
        Ok(Self {
            state: State::Done,
            ..self
        })
    }
}

impl<I, O> Agent for OnDemandClient<I, O>
where
    I: IntoIterator<Item = Point>,
    O: Observer,
{
    type Message = Message;
    type State = State;

    fn state(&self) -> &Self::State {
        &self.state
    }

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

    fn build_next(&self) -> Self::Message {
        match (&self.state, &self.next) {
            (State::Idle, Some(range)) => Message::RequestRange {
                range: range.clone(),
            },
            (State::Idle, None) => panic!("I'm idle but no more block requests available"),
            _ => panic!("I don't have agency, don't know what to do"),
        }
    }

    fn apply_start(self) -> Transition<Self> {
        self.wait_for_request()
    }

    fn apply_outbound(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::Idle, Message::RequestRange { range }) => self.on_range_requested(range),
            (State::Idle, Message::ClientDone) => self.on_client_done(),
            _ => panic!("I don't have agency, I don't expect outbound message"),
        }
    }

    fn apply_inbound(self, msg: Self::Message) -> Transition<Self> {
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
            (State::Streaming, Message::BatchDone) => self.on_batch_done(),
            _ => panic!("I have agency, I don't expect inbound message"),
        }
    }
}
