use pallas_codec::utils::AnyCbor;
use std::fmt::Debug;
use thiserror::*;

use super::{AcquireFailure, Message, State};
use crate::miniprotocols::Point;
use crate::multiplexer;

/// Errors produced by the local-state-query server agent.
#[derive(Error, Debug)]
pub enum Error {
    /// Tried to receive while we hold agency.
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,
    /// Tried to send while the peer holds agency.
    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,
    /// Inbound message is not valid for the current state.
    #[error("inbound message is not valid for current state")]
    InvalidInbound,
    /// Outbound message is not valid for the current state.
    #[error("outbound message is not valid for current state")]
    InvalidOutbound,
    /// Underlying multiplexer error.
    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

/// Request received from the client to acquire the ledger.
pub struct ClientAcquireRequest(pub Option<Point>);

/// Request received from the client while in the Acquired state.
#[derive(Debug)]
pub enum ClientQueryRequest {
    /// Drop the current snapshot and acquire a new one.
    ReAcquire(Option<Point>),
    /// Run a query against the current snapshot.
    Query(AnyCbor),
    /// Release the current snapshot.
    Release,
}

/// Local-state-query server agent.
pub struct GenericServer(State, multiplexer::ChannelBuffer);

impl GenericServer {
    /// Build a server over a freshly subscribed agent channel.
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(State::Idle, multiplexer::ChannelBuffer::new(channel))
    }

    /// Current state-machine state.
    pub fn state(&self) -> &State {
        &self.0
    }

    /// True if the protocol has terminated.
    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    fn has_agency(&self) -> bool {
        matches!(self.state(), State::Acquiring | State::Querying)
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

    fn assert_outbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Acquiring, Message::Acquired) => Ok(()),
            (State::Acquiring, Message::Failure(_)) => Ok(()),
            (State::Querying, Message::Result(_)) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Idle, Message::Acquire(_)) => Ok(()),
            (State::Idle, Message::Done) => Ok(()),
            (State::Acquired, Message::Query(_)) => Ok(()),
            (State::Acquired, Message::ReAcquire(_)) => Ok(()),
            (State::Acquired, Message::Release) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    /// Low-level send.
    pub async fn send_message(&mut self, msg: &Message) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1.send_msg_chunks(msg).await.map_err(Error::Plexer)?;

        Ok(())
    }

    /// Low-level receive.
    pub async fn recv_message(&mut self) -> Result<Message, Error> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(Error::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    /// Reject the pending acquire with the given reason.
    pub async fn send_failure(&mut self, reason: AcquireFailure) -> Result<(), Error> {
        let msg = Message::Failure(reason);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    /// Confirm the pending acquire.
    pub async fn send_acquired(&mut self) -> Result<(), Error> {
        let msg = Message::Acquired;
        self.send_message(&msg).await?;
        self.0 = State::Acquired;

        Ok(())
    }

    /// Reply to the pending query with the given CBOR-encoded result.
    pub async fn send_result(&mut self, response: AnyCbor) -> Result<(), Error> {
        let msg = Message::Result(response);
        self.send_message(&msg).await?;
        self.0 = State::Acquired;

        Ok(())
    }

    /// Wait for the next request while the protocol is in the `Idle` state.
    /// Returns `None` if the client terminated the protocol.
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

    /// Wait for the next request while the protocol is in the `Acquired` state.
    pub async fn recv_while_acquired(&mut self) -> Result<ClientQueryRequest, Error> {
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

/// Concrete local-state-query server (default instantiation).
pub type Server = GenericServer;
