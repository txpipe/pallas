use std::fmt::Debug;

use log::{debug, log_enabled, trace};

use pallas_machines::{
    primitives::Point, Agent, DecodePayload, EncodePayload, MachineError, MachineOutput, Transition,
};

use crate::{Message, State, Tip};

/// A trait to deal with polymorphic payloads in the ChainSync protocol
/// (WrappedHeader vs BlockBody)
pub trait BlockLike: EncodePayload + DecodePayload + Debug {
    fn block_point(&self) -> Result<Point, Box<dyn std::error::Error>>;
}

/// An observer of chain-sync events sent by the state-machine
pub trait Observer<C>
where
    C: Debug,
{
    fn on_block(
        &self,
        cursor: &Option<Point>,
        content: &C,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!(
            "asked to save block content {:?} at cursor {:?}",
            content,
            cursor
        );
        Ok(())
    }

    fn on_intersect_found(
        &self,
        point: &Point,
        tip: &Tip,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("intersect was found {:?} (tip: {:?})", point, tip);
        Ok(())
    }

    fn on_rollback(&self, point: &Point) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("asked to roll back {:?}", point);
        Ok(())
    }
    fn on_tip_reached(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("tip was reached");
        Ok(())
    }
}

#[derive(Debug)]
pub struct NoopObserver {}

impl<C> Observer<C> for NoopObserver where C: Debug {}

#[derive(Debug)]
pub struct Consumer<C, O>
where
    O: Observer<C>,
    C: Debug,
{
    pub state: State,
    pub known_points: Vec<Point>,
    pub cursor: Option<Point>,
    pub tip: Option<Tip>,

    observer: O,

    // as recommended here: https://doc.rust-lang.org/error-index.html#E0207
    _phantom: Option<C>,
}

impl<C, O> Consumer<C, O>
where
    C: BlockLike + EncodePayload + DecodePayload + Debug,
    O: Observer<C>,
{
    pub fn initial(known_points: Vec<Point>, observer: O) -> Self {
        Self {
            state: State::Idle,
            cursor: None,
            tip: None,
            known_points,
            observer,

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

        self.observer.on_intersect_found(&point, &tip)?;

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

        let point = content.block_point()?;

        if log_enabled!(log::Level::Trace) {
            trace!("content: {:?}", content);
        }

        debug!("reporint block to observer");
        self.observer.on_block(&self.cursor, &content)?;

        Ok(Self {
            cursor: Some(point),
            tip: Some(tip),
            state: State::Idle,
            ..self
        })
    }

    fn on_roll_backward(self, point: Point, tip: Tip) -> Transition<Self> {
        debug!("rolling backward to point: {:?}", point);

        debug!("reporting rollback to observer");
        self.observer.on_rollback(&point)?;

        Ok(Self {
            tip: Some(tip),
            cursor: Some(point),
            state: State::Idle,
            ..self
        })
    }

    fn on_await_reply(self) -> Transition<Self> {
        debug!("reached tip, await reply");

        debug!("reporting tip to observer");
        self.observer.on_tip_reached()?;

        Ok(Self {
            state: State::MustReply,
            ..self
        })
    }
}

impl<C, O> Agent for Consumer<C, O>
where
    C: BlockLike + EncodePayload + DecodePayload + Debug + 'static,
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
            (State::CanAwait, Message::AwaitReply) => self.on_await_reply(),
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
        let msg = Message::<NoopContent>::FindIntersect(vec![self.wellknown_point.clone()]);

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

#[derive(Debug)]
pub struct NoopContent {}

impl EncodePayload for NoopContent {
    fn encode_payload(
        &self,
        _e: &mut pallas_machines::PayloadEncoder,
    ) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }
}

impl DecodePayload for NoopContent {
    fn decode_payload(
        _d: &mut pallas_machines::PayloadDecoder,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        todo!()
    }
}

impl BlockLike for NoopContent {
    fn block_point(&self) -> Result<Point, Box<dyn std::error::Error>> {
        todo!()
    }
}

impl Agent for TipFinder {
    type Message = Message<NoopContent>;

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
