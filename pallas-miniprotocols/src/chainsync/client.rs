use pallas_codec::Fragment;
use pallas_multiplexer::agents::{Channel, ChannelBuffer, ChannelError};
use std::marker::PhantomData;
use thiserror::Error;
use tracing::debug;

use crate::common::Point;

use super::{BlockContent, HeaderContent, Message, State, Tip};

#[derive(Error, Debug)]
pub enum Error {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    #[error("no intersection point found")]
    IntersectionNotFound,

    #[error("error while sending or receiving data through the channel")]
    ChannelError(ChannelError),
}

pub type IntersectResponse = (Option<Point>, Tip);

#[derive(Debug)]
pub enum NextResponse<CONTENT> {
    RollForward(CONTENT, Tip),
    RollBackward(Point, Tip),
    Await,
}

pub struct Client<H, O>(State, ChannelBuffer<H>, PhantomData<O>)
where
    H: Channel,
    Message<O>: Fragment;

impl<H, O> Client<H, O>
where
    H: Channel,
    Message<O>: Fragment,
{
    pub fn new(channel: H) -> Self {
        Self(State::Idle, ChannelBuffer::new(channel), PhantomData {})
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    pub fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => true,
            State::CanAwait => false,
            State::MustReply => false,
            State::Intersect => false,
            State::Done => false,
        }
    }

    fn assert_agency_is_ours(&self) -> Result<(), Error> {
        if !self.has_agency() {
            Err(Error::AgencyIsTheirs)
        } else {
            Ok(())
        }
    }

    fn assert_agency_is_theirs(&self) -> Result<(), Error> {
        if self.has_agency() {
            Err(Error::AgencyIsOurs)
        } else {
            Ok(())
        }
    }

    fn assert_outbound_state(&self, msg: &Message<O>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Idle, Message::RequestNext) => Ok(()),
            (State::Idle, Message::FindIntersect(_)) => Ok(()),
            (State::Idle, Message::Done) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message<O>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::CanAwait, Message::RollForward(_, _)) => Ok(()),
            (State::CanAwait, Message::RollBackward(_, _)) => Ok(()),
            (State::CanAwait, Message::AwaitReply) => Ok(()),
            (State::MustReply, Message::RollForward(_, _)) => Ok(()),
            (State::MustReply, Message::RollBackward(_, _)) => Ok(()),
            (State::Intersect, Message::IntersectFound(_, _)) => Ok(()),
            (State::Intersect, Message::IntersectNotFound(_)) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn send_message(&mut self, msg: &Message<O>) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;

        self.1.send_msg_chunks(msg).map_err(Error::ChannelError)?;

        Ok(())
    }

    pub fn recv_message(&mut self) -> Result<Message<O>, Error> {
        self.assert_agency_is_theirs()?;

        let msg = self.1.recv_full_msg().map_err(Error::ChannelError)?;

        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub fn send_find_intersect(&mut self, points: Vec<Point>) -> Result<(), Error> {
        let msg = Message::FindIntersect(points);
        self.send_message(&msg)?;
        self.0 = State::Intersect;

        Ok(())
    }

    pub fn recv_intersect_response(&mut self) -> Result<IntersectResponse, Error> {
        match self.recv_message()? {
            Message::IntersectFound(point, tip) => {
                self.0 = State::Idle;
                Ok((Some(point), tip))
            }
            Message::IntersectNotFound(tip) => {
                self.0 = State::Idle;
                Ok((None, tip))
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn find_intersect(&mut self, points: Vec<Point>) -> Result<IntersectResponse, Error> {
        self.send_find_intersect(points)?;
        self.recv_intersect_response()
    }

    pub fn send_request_next(&mut self) -> Result<(), Error> {
        let msg = Message::RequestNext;
        self.send_message(&msg)?;
        self.0 = State::CanAwait;

        Ok(())
    }

    pub fn recv_while_can_await(&mut self) -> Result<NextResponse<O>, Error> {
        match self.recv_message()? {
            Message::AwaitReply => {
                self.0 = State::MustReply;
                Ok(NextResponse::Await)
            }
            Message::RollForward(a, b) => {
                self.0 = State::Idle;
                Ok(NextResponse::RollForward(a, b))
            }
            Message::RollBackward(a, b) => {
                self.0 = State::Idle;
                Ok(NextResponse::RollBackward(a, b))
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn recv_while_must_reply(&mut self) -> Result<NextResponse<O>, Error> {
        match self.recv_message()? {
            Message::RollForward(a, b) => {
                self.0 = State::Idle;
                Ok(NextResponse::RollForward(a, b))
            }
            Message::RollBackward(a, b) => {
                self.0 = State::Idle;
                Ok(NextResponse::RollBackward(a, b))
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn request_next(&mut self) -> Result<NextResponse<O>, Error> {
        debug!("requesting next block");

        self.send_request_next()?;

        self.recv_while_can_await()
    }

    pub fn intersect_origin(&mut self) -> Result<Point, Error> {
        debug!("intersecting origin");

        let (point, _) = self.find_intersect(vec![Point::Origin])?;

        point.ok_or(Error::IntersectionNotFound)
    }

    pub fn intersect_tip(&mut self) -> Result<Point, Error> {
        let (_, Tip(point, _)) = self.find_intersect(vec![Point::Origin])?;

        debug!(?point, "found tip value");

        let (point, _) = self.find_intersect(vec![point])?;

        point.ok_or(Error::IntersectionNotFound)
    }

    pub fn send_done(&mut self) -> Result<(), Error> {
        let msg = Message::Done;
        self.send_message(&msg)?;
        self.0 = State::Done;

        Ok(())
    }
}

pub type N2NClient<H> = Client<H, HeaderContent>;

pub type N2CClient<H> = Client<H, BlockContent>;
