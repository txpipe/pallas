use std::fmt::Debug;

use pallas_codec::Fragment;

use std::marker::PhantomData;
use thiserror::*;

use super::{AcquireFailure, Message, Query, State};
use crate::miniprotocols::Point;
use crate::plexer;

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
    #[error("failure acquiring point, not found")]
    AcquirePointNotFound,
    #[error("failure acquiring point, too old")]
    AcquirePointTooOld,
    #[error("error while sending or receiving data through the channel")]
    Plexer(plexer::Error),
}

impl From<AcquireFailure> for Error {
    fn from(x: AcquireFailure) -> Self {
        match x {
            AcquireFailure::PointTooOld => Error::AcquirePointTooOld,
            AcquireFailure::PointNotOnChain => Error::AcquirePointNotFound,
        }
    }
}

pub struct Client<Q>(State, plexer::ChannelBuffer, PhantomData<Q>)
where
    Q: Query,
    Message<Q>: Fragment;

impl<Q> Client<Q>
where
    Q: Query,
    Message<Q>: Fragment,
{
    pub fn new(channel: plexer::AgentChannel) -> Self {
        Self(
            State::Idle,
            plexer::ChannelBuffer::new(channel),
            PhantomData {},
        )
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

    pub async fn send_message(&mut self, msg: &Message<Q>) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1.send_msg_chunks(msg).await.map_err(Error::Plexer)?;

        Ok(())
    }

    pub async fn recv_message(&mut self) -> Result<Message<Q>, Error> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(Error::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub async fn send_acquire(&mut self, point: Option<Point>) -> Result<(), Error> {
        let msg = Message::<Q>::Acquire(point);
        self.send_message(&msg).await?;
        self.0 = State::Acquiring;

        Ok(())
    }

    pub async fn recv_while_acquiring(&mut self) -> Result<(), Error> {
        match self.recv_message().await? {
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

    pub async fn acquire(&mut self, point: Option<Point>) -> Result<(), Error> {
        self.send_acquire(point).await?;
        self.recv_while_acquiring().await
    }

    pub async fn send_query(&mut self, request: Q::Request) -> Result<(), Error> {
        let msg = Message::<Q>::Query(request);
        self.send_message(&msg).await?;
        self.0 = State::Querying;

        Ok(())
    }

    pub async fn recv_while_querying(&mut self) -> Result<Q::Response, Error> {
        match self.recv_message().await? {
            Message::Result(x) => {
                self.0 = State::Acquired;
                Ok(x)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn query(&mut self, request: Q::Request) -> Result<Q::Response, Error> {
        self.send_query(request).await?;
        self.recv_while_querying().await
    }
}

pub type ClientV10 = Client<super::queries::QueryV10>;
