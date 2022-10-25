use std::fmt::Debug;

use pallas_codec::Fragment;

use crate::common::Point;

use pallas_multiplexer::agents::{Channel, ChannelBuffer, ChannelError};
use std::marker::PhantomData;
use thiserror::*;

use super::{AcquireFailure, Message, Query, State};

#[derive(Error, Debug)]
pub enum Error {
    #[error("attemted to receive message while agency is ours")]
    AgencyIsOurs,
    #[error("attemted to send message while agency is theirs")]
    AgencyIsTheirs,
    #[error("inbound message is not valid for current state")]
    InvalidInbound,
    #[error("outbound message is not valid for current state")]
    InvalidOutbound,
    #[error("failure acquiring point, not found")]
    AcquirePointNotFound,
    #[error("failure acquiring point, too old")]
    AcquirePointTooOld,
    #[error("error while sending or receiving data through the channel")]
    ChannelError(ChannelError),
}

impl From<AcquireFailure> for Error {
    fn from(x: AcquireFailure) -> Self {
        match x {
            AcquireFailure::PointTooOld => Error::AcquirePointTooOld,
            AcquireFailure::PointNotInChain => Error::AcquirePointNotFound,
        }
    }
}

pub struct Client<H, Q>(State, ChannelBuffer<H>, PhantomData<Q>)
where
    H: Channel,
    Q: Query,
    Message<Q>: Fragment;

impl<H, Q> Client<H, Q>
where
    H: Channel,
    Q: Query,
    Message<Q>: Fragment,
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

    #[allow(clippy::match_like_matches_macro)]
    fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => true,
            State::Acquired => true,
            _ => false,
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

    fn assert_outbound_state(&self, msg: &Message<Q>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Idle, Message::Acquire(_)) => Ok(()),
            (State::Idle, Message::Done) => Ok(()),
            (State::Acquired, Message::Query(_)) => Ok(()),
            (State::Acquired, Message::Release) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message<Q>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Acquiring, Message::Acquired) => Ok(()),
            (State::Acquiring, Message::Failure(_)) => Ok(()),
            (State::Querying, Message::Result(_)) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn send_message(&mut self, msg: &Message<Q>) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1.send_msg_chunks(msg).map_err(Error::ChannelError)?;

        Ok(())
    }

    pub fn recv_message(&mut self) -> Result<Message<Q>, Error> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().map_err(Error::ChannelError)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub fn send_acquire(&mut self, point: Option<Point>) -> Result<(), Error> {
        let msg = Message::<Q>::Acquire(point);
        self.send_message(&msg)?;
        self.0 = State::Acquiring;

        Ok(())
    }

    pub fn recv_acquiring(&mut self) -> Result<(), Error> {
        match self.recv_message()? {
            Message::Acquired => {
                self.0 = State::Acquired;
                Ok(())
            }
            Message::Failure(x) => {
                self.0 = State::Idle;
                Err(Error::from(x))
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn acquire(&mut self, point: Option<Point>) -> Result<(), Error> {
        self.send_acquire(point)?;
        self.recv_acquiring()
    }

    pub fn send_query(&mut self, request: Q::Request) -> Result<(), Error> {
        let msg = Message::<Q>::Query(request);
        self.send_message(&msg)?;
        self.0 = State::Querying;

        Ok(())
    }

    pub fn recv_querying(&mut self) -> Result<Q::Response, Error> {
        match self.recv_message()? {
            Message::Result(x) => {
                self.0 = State::Acquired;
                Ok(x)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn query(&mut self, request: Q::Request) -> Result<Q::Response, Error> {
        self.send_query(request)?;
        self.recv_querying()
    }
}

pub type ClientV10<H> = Client<H, super::queries::QueryV10>;
