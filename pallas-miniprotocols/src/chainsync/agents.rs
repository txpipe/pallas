use core::panic;
use std::fmt::Debug;
use std::marker::PhantomData;

use pallas_codec::Fragment;

use crate::machines::{Agent, MachineError, Transition};

use crate::common::Point;

use super::{BlockContent, HeaderContent, Message, SkippedContent, State, Tip};

#[derive(Debug, PartialEq, Eq)]
pub enum Continuation {
    Proceed,
    DropOut,
    Done,
}

/// An observer of chain-sync events sent by the state-machine
pub trait Observer<C> {
    fn on_roll_forward(
        &mut self,
        _content: C,
        tip: &Tip,
    ) -> Result<Continuation, Box<dyn std::error::Error>> {
        log::debug!("asked to roll forward, tip at {:?}", tip);

        Ok(Continuation::Proceed)
    }

    fn on_intersect_found(
        &mut self,
        point: &Point,
        tip: &Tip,
    ) -> Result<Continuation, Box<dyn std::error::Error>> {
        log::debug!("intersect was found {:?} (tip: {:?})", point, tip);

        Ok(Continuation::Proceed)
    }

    fn on_rollback(&mut self, point: &Point) -> Result<Continuation, Box<dyn std::error::Error>> {
        log::debug!("asked to roll back {:?}", point);

        Ok(Continuation::Proceed)
    }

    fn on_tip_reached(&mut self) -> Result<Continuation, Box<dyn std::error::Error>> {
        log::debug!("tip was reached");

        Ok(Continuation::Proceed)
    }
}

#[derive(Debug)]
pub struct NoopObserver {}

impl<C> Observer<C> for NoopObserver {}

#[derive(Debug)]
pub struct Consumer<C, O>
where
    Self: Agent,
    O: Observer<C>,
{
    pub state: State,
    pub known_points: Option<Vec<Point>>,
    pub intersect: Option<Point>,
    pub tip: Option<Tip>,

    continuation: Continuation,

    observer: O,

    _phantom: PhantomData<C>,
}

impl<C, O> Consumer<C, O>
where
    O: Observer<C>,
    Message<C>: Fragment,
    C: std::fmt::Debug + 'static,
{
    pub fn initial(known_points: Option<Vec<Point>>, observer: O) -> Self {
        Self {
            state: State::Idle,
            intersect: None,
            tip: None,
            known_points,
            continuation: Continuation::Proceed,
            observer,
            _phantom: PhantomData::default(),
        }
    }

    fn on_intersect_found(mut self, point: Point, tip: Tip) -> Transition<Self> {
        log::debug!("intersect found: {:?} (tip: {:?})", point, tip);

        let continuation = self
            .observer
            .on_intersect_found(&point, &tip)
            .map_err(MachineError::downstream)?;

        Ok(Self {
            tip: Some(tip),
            intersect: Some(point),
            state: State::Idle,
            continuation,
            ..self
        })
    }

    fn on_intersect_not_found(self, tip: Tip) -> Transition<Self> {
        log::debug!("intersect not found (tip: {:?})", tip);

        Ok(Self {
            tip: Some(tip),
            intersect: None,
            state: State::Done,
            ..self
        })
    }

    fn on_roll_forward(mut self, content: C, tip: Tip) -> Transition<Self> {
        log::debug!("rolling forward");

        let continuation = self
            .observer
            .on_roll_forward(content, &tip)
            .map_err(MachineError::downstream)?;

        Ok(Self {
            tip: Some(tip),
            state: State::Idle,
            continuation,
            ..self
        })
    }

    fn on_roll_backward(mut self, point: Point, tip: Tip) -> Transition<Self> {
        log::debug!("rolling backward to point: {:?}", point);

        let continuation = self
            .observer
            .on_rollback(&point)
            .map_err(MachineError::downstream)?;

        Ok(Self {
            tip: Some(tip),
            intersect: Some(point),
            state: State::Idle,
            continuation,
            ..self
        })
    }

    fn on_await_reply(mut self) -> Transition<Self> {
        log::debug!("reached tip, await reply");

        let continuation = self
            .observer
            .on_tip_reached()
            .map_err(MachineError::downstream)?;

        Ok(Self {
            state: State::MustReply,
            continuation,
            ..self
        })
    }
}

impl<C, O> Agent for Consumer<C, O>
where
    O: Observer<C>,
    C: Debug + 'static,
    Message<C>: Fragment,
{
    type Message = Message<C>;
    type State = State;

    fn state(&self) -> &Self::State {
        &self.state
    }

    fn is_done(&self) -> bool {
        self.state == State::Done || self.continuation == Continuation::DropOut
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

    fn build_next(&self) -> Self::Message {
        match (&self.state, &self.intersect, &self.continuation) {
            (State::Idle, _, Continuation::Done) => Message::<C>::Done,
            (State::Idle, None, Continuation::Proceed) => match &self.known_points {
                Some(x) => Message::<C>::FindIntersect(x.clone()),
                None => Message::<C>::RequestNext,
            },
            (State::Idle, Some(_), Continuation::Proceed) => Message::<C>::RequestNext,
            _ => panic!(""),
        }
    }

    fn apply_start(self) -> Transition<Self> {
        Ok(self)
    }

    fn apply_outbound(self, msg: Self::Message) -> Transition<Self> {
        match (self.state, msg) {
            (State::Idle, Message::RequestNext) => Ok(Self {
                state: State::CanAwait,
                ..self
            }),
            (State::Idle, Message::FindIntersect(_)) => Ok(Self {
                state: State::Intersect,
                ..self
            }),
            (State::Idle, Message::Done) => Ok(Self {
                state: State::Done,
                ..self
            }),
            _ => panic!(""),
        }
    }

    fn apply_inbound(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::CanAwait, Message::RollForward(header, tip)) => {
                self.on_roll_forward(header, tip)
            }
            (State::CanAwait, Message::RollBackward(point, tip)) => {
                self.on_roll_backward(point, tip)
            }
            (State::CanAwait, Message::AwaitReply) => self.on_await_reply(),
            (State::MustReply, Message::RollForward(content, tip)) => {
                self.on_roll_forward(content, tip)
            }
            (State::MustReply, Message::RollBackward(point, tip)) => {
                self.on_roll_backward(point, tip)
            }
            (State::Intersect, Message::IntersectFound(point, tip)) => {
                self.on_intersect_found(point, tip)
            }
            (State::Intersect, Message::IntersectNotFound(tip)) => self.on_intersect_not_found(tip),
            (state, msg) => Err(MachineError::invalid_msg::<Self>(state, &msg)),
        }
    }
}

#[derive(Debug)]
pub struct TipFinder {
    pub state: State,
    pub wellknown_point: Point,
    pub output: Option<Tip>,
}

impl TipFinder {
    pub fn initial(wellknown_point: Point) -> Self {
        TipFinder {
            wellknown_point,
            output: None,
            state: State::Idle,
        }
    }

    fn on_intersect_found(self, tip: Tip) -> Transition<Self> {
        log::debug!("intersect found with tip: {:?}", tip);

        Ok(Self {
            state: State::Done,
            output: Some(tip),
            ..self
        })
    }

    fn on_intersect_not_found(self, tip: Tip) -> Transition<Self> {
        log::debug!("intersect not found but still have a tip: {:?}", tip);

        Ok(Self {
            state: State::Done,
            output: Some(tip),
            ..self
        })
    }
}

pub type HeaderConsumer<O> = Consumer<HeaderContent, O>;

pub type BlockConsumer<O> = Consumer<BlockContent, O>;

impl Agent for TipFinder {
    type Message = Message<SkippedContent>;
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
            State::CanAwait => false,
            State::MustReply => false,
            State::Intersect => false,
            State::Done => false,
        }
    }

    fn build_next(&self) -> Self::Message {
        match self.state {
            State::Idle => {
                Message::<SkippedContent>::FindIntersect(vec![self.wellknown_point.clone()])
            }
            _ => panic!("I don't know what to do"),
        }
    }

    fn apply_start(self) -> Transition<Self> {
        Ok(self)
    }

    fn apply_outbound(self, msg: Self::Message) -> Transition<Self> {
        match (self.state, msg) {
            (State::Idle, Message::FindIntersect(_)) => Ok(Self {
                state: State::Intersect,
                ..self
            }),
            _ => panic!("I don't know what to do"),
        }
    }

    fn apply_inbound(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::Intersect, Message::IntersectFound(_point, tip)) => {
                self.on_intersect_found(tip)
            }
            (State::Intersect, Message::IntersectNotFound(tip)) => self.on_intersect_not_found(tip),
            (state, msg) => Err(MachineError::invalid_msg::<Self>(state, &msg)),
        }
    }
}
