use std::fmt::Debug;
use std::marker::PhantomData;

use log::debug;

use crate::machines::{Agent, MachineError, MachineOutput, Transition};
use crate::{DecodePayload, EncodePayload};

use crate::common::Point;

use super::{BlockContent, HeaderContent, Message, SkippedContent, State, Tip};

/// An observer of chain-sync events sent by the state-machine
pub trait Observer<C> {
    fn on_roll_forward(
        &mut self,
        _content: C,
        tip: &Tip,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("asked to roll forward, tip at {:?}", tip);

        Ok(())
    }

    fn on_intersect_found(
        &mut self,
        point: &Point,
        tip: &Tip,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("intersect was found {:?} (tip: {:?})", point, tip);
        Ok(())
    }

    fn on_rollback(&mut self, point: &Point) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("asked to roll back {:?}", point);
        Ok(())
    }
    fn on_tip_reached(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("tip was reached");
        Ok(())
    }
}

#[derive(Debug)]
pub struct NoopObserver {}

impl<C> Observer<C> for NoopObserver {}

#[derive(Debug)]
pub struct Consumer<C, O>
where
    O: Observer<C>,
{
    pub state: State,
    pub known_points: Option<Vec<Point>>,
    pub intersect: Option<Point>,
    pub tip: Option<Tip>,

    observer: O,

    _phantom: PhantomData<C>,
}

impl<C, O> Consumer<C, O>
where
    O: Observer<C>,
    C: DecodePayload + EncodePayload,
{
    pub fn initial(known_points: Option<Vec<Point>>, observer: O) -> Self {
        Self {
            state: State::Idle,
            intersect: None,
            tip: None,
            known_points,
            observer,
            _phantom: PhantomData::default(),
        }
    }

    fn send_find_intersect(self, tx: &impl MachineOutput) -> Transition<Self> {
        debug!("requesting find intersect");

        let points = match &self.known_points {
            Some(x) => x.clone(),
            None => return Err("can't find intersect without known points".into()),
        };

        let msg = Message::<C>::FindIntersect(points);

        tx.send_msg(&msg)?;

        Ok(Self {
            state: State::Intersect,
            ..self
        })
    }

    fn send_request_next(self, tx: &impl MachineOutput) -> Transition<Self> {
        debug!("requesting next");

        let msg = Message::<C>::RequestNext;

        tx.send_msg(&msg)?;

        Ok(Self {
            state: State::CanAwait,
            ..self
        })
    }

    fn on_intersect_found(mut self, point: Point, tip: Tip) -> Transition<Self> {
        debug!("intersect found: {:?} (tip: {:?})", point, tip);

        self.observer.on_intersect_found(&point, &tip)?;

        Ok(Self {
            tip: Some(tip),
            intersect: Some(point),
            state: State::Idle,
            ..self
        })
    }

    fn on_intersect_not_found(self, tip: Tip) -> Transition<Self> {
        debug!("intersect not found (tip: {:?})", tip);

        Ok(Self {
            tip: Some(tip),
            intersect: None,
            state: State::Done,
            ..self
        })
    }

    fn on_roll_forward(mut self, content: C, tip: Tip) -> Transition<Self> {
        debug!("rolling forward");

        self.observer.on_roll_forward(content, &tip)?;

        Ok(Self {
            tip: Some(tip),
            state: State::Idle,
            ..self
        })
    }

    fn on_roll_backward(mut self, point: Point, tip: Tip) -> Transition<Self> {
        debug!("rolling backward to point: {:?}", point);

        debug!("reporting rollback to observer");
        self.observer.on_rollback(&point)?;

        Ok(Self {
            tip: Some(tip),
            intersect: Some(point),
            state: State::Idle,
            ..self
        })
    }

    fn on_await_reply(mut self) -> Transition<Self> {
        debug!("reached tip, await reply");

        self.observer.on_tip_reached()?;

        Ok(Self {
            state: State::MustReply,
            ..self
        })
    }
}

impl<C, O> Agent for Consumer<C, O>
where
    C: EncodePayload + DecodePayload + Debug + 'static,
    O: Observer<C>,
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
            State::Idle => match self.intersect {
                // keep going from pointer
                Some(_) => self.send_request_next(tx),
                _ => match self.known_points {
                    // need to find instersection first
                    Some(_) => self.send_find_intersect(tx),
                    // start from genesis
                    None => self.send_request_next(tx),
                },
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
            (_, msg) => Err(MachineError::InvalidMsgForState(self.state, msg).into()),
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

    fn send_find_intersect(self, tx: &impl MachineOutput) -> Transition<Self> {
        let msg = Message::<SkippedContent>::FindIntersect(vec![self.wellknown_point.clone()]);

        tx.send_msg(&msg)?;

        Ok(Self {
            state: State::Intersect,
            ..self
        })
    }

    fn on_intersect_found(self, tip: Tip) -> Transition<Self> {
        debug!("intersect found with tip: {:?}", tip);

        Ok(Self {
            state: State::Done,
            output: Some(tip),
            ..self
        })
    }

    fn on_intersect_not_found(self, tip: Tip) -> Transition<Self> {
        debug!("intersect not found but still have a tip: {:?}", tip);

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
            State::Idle => self.send_find_intersect(tx),
            _ => panic!("I don't have agency, don't know what to do"),
        }
    }

    fn receive_next(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::Intersect, Message::IntersectFound(_point, tip)) => {
                self.on_intersect_found(tip)
            }
            (State::Intersect, Message::IntersectNotFound(tip)) => self.on_intersect_not_found(tip),
            (_, msg) => Err(MachineError::InvalidMsgForState(self.state, msg).into()),
        }
    }
}
