use std::fmt::Debug;

use pallas_codec::Fragment;

use std::marker::PhantomData;
use thiserror::*;

use super::{AcquireFailure, Message, Query, State};
use crate::miniprotocols::Point;
use crate::multiplexer;

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
    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

/// Request received from the client to acquire the ledger
pub struct ClientAcquireRequest(pub Option<Point>);

/// Request received from the client when in the Acquired state
#[derive(Debug)]
pub enum ClientQueryRequest<Q: Query> {
    ReAcquire(Option<Point>),
    Query(Q::Request),
    Release,
}

pub struct GenericServer<Q>(State, multiplexer::ChannelBuffer, PhantomData<Q>)
where
    Q: Query,
    Message<Q>: Fragment;

impl<Q> GenericServer<Q>
where
    Q: Query,
    Message<Q>: Fragment,
{
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(
            State::Idle,
            multiplexer::ChannelBuffer::new(channel),
            PhantomData {},
        )
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state() {
            State::Acquiring => true,
            State::Querying => true,
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
            (State::Acquiring, Message::Acquired) => Ok(()),
            (State::Acquiring, Message::Failure(_)) => Ok(()),
            (State::Querying, Message::Response(_)) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message<Q>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Idle, Message::Acquire(_)) => Ok(()),
            (State::Idle, Message::Done) => Ok(()),
            (State::Acquired, Message::Query(_)) => Ok(()),
            (State::Acquired, Message::ReAcquire(_)) => Ok(()),
            (State::Acquired, Message::Release) => Ok(()),
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

    pub async fn send_failure(&mut self, reason: AcquireFailure) -> Result<(), Error> {
        let msg = Message::<Q>::Failure(reason);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn send_acquired(&mut self) -> Result<(), Error> {
        let msg = Message::<Q>::Acquired;
        self.send_message(&msg).await?;
        self.0 = State::Acquired;

        Ok(())
    }

    pub async fn send_result(&mut self, response: Q::Response) -> Result<(), Error> {
        let msg = Message::<Q>::Response(response);
        self.send_message(&msg).await?;
        self.0 = State::Acquired;

        Ok(())
    }

    /// Receive a message from the Client when the protocol is in the Idle state
    ///
    /// Returns the client's request to acquire the ledger or None if a Done
    /// message was received from the client causing the protocol to finish.
    pub async fn recv_while_idle(&mut self) -> Result<Option<ClientAcquireRequest>, Error> {
        match self.recv_message().await? {
            Message::Acquire(point) => {
                self.0 = State::Acquiring;
                Ok(Some(ClientAcquireRequest(point)))
            }
            Message::Done => {
                self.0 = State::Done;
                Ok(None)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn recv_while_acquired(&mut self) -> Result<ClientQueryRequest<Q>, Error> {
        match self.recv_message().await? {
            Message::ReAcquire(point) => {
                self.0 = State::Acquiring;
                Ok(ClientQueryRequest::ReAcquire(point))
            }
            Message::Query(query) => {
                self.0 = State::Querying;
                Ok(ClientQueryRequest::Query(query))
            }
            Message::Release => {
                self.0 = State::Idle;
                Ok(ClientQueryRequest::Release)
            }
            _ => Err(Error::InvalidInbound),
        }
    }
}

pub type Server = GenericServer<super::queries::QueryV16>;
