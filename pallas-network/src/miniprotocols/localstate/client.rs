use pallas_codec::utils::AnyCbor;
use std::fmt::Debug;
use thiserror::*;

use super::{AcquireFailure, Message, State};
use crate::miniprotocols::Point;
use crate::multiplexer;

/// Errors produced by the local-state-query client agent.
#[derive(Error, Debug)]
pub enum ClientError {
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

    /// Requested point is not on the server's current chain.
    #[error("failure acquiring point, not found")]
    AcquirePointNotFound,

    /// Requested point is older than the server's rolling window.
    #[error("failure acquiring point, too old")]
    AcquirePointTooOld,

    /// Query result could not be CBOR-decoded.
    #[error("failure decoding CBOR data")]
    InvalidCbor(pallas_codec::minicbor::decode::Error),

    /// Underlying multiplexer error.
    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

impl From<AcquireFailure> for ClientError {
    fn from(x: AcquireFailure) -> Self {
        match x {
            AcquireFailure::PointTooOld => ClientError::AcquirePointTooOld,
            AcquireFailure::PointNotOnChain => ClientError::AcquirePointNotFound,
        }
    }
}

/// Local-state-query client agent.
pub struct GenericClient(State, multiplexer::ChannelBuffer);

impl GenericClient {
    /// Build a client over a freshly subscribed agent channel.
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

    #[allow(clippy::match_like_matches_macro)]
    fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => true,
            State::Acquired => true,
            _ => false,
        }
    }

    fn assert_agency_is_ours(&self) -> Result<(), ClientError> {
        if !self.has_agency() {
            Err(ClientError::AgencyIsTheirs)
        } else {
            Ok(())
        }
    }

    fn assert_agency_is_theirs(&self) -> Result<(), ClientError> {
        if self.has_agency() {
            Err(ClientError::AgencyIsOurs)
        } else {
            Ok(())
        }
    }

    fn assert_outbound_state(&self, msg: &Message) -> Result<(), ClientError> {
        match (&self.0, msg) {
            (State::Idle, Message::Acquire(_)) => Ok(()),
            (State::Idle, Message::Done) => Ok(()),
            (State::Acquired, Message::Query(_)) => Ok(()),
            (State::Acquired, Message::ReAcquire(_)) => Ok(()),
            (State::Acquired, Message::Release) => Ok(()),
            _ => Err(ClientError::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), ClientError> {
        match (&self.0, msg) {
            (State::Acquiring, Message::Acquired) => Ok(()),
            (State::Acquiring, Message::Failure(_)) => Ok(()),
            (State::Querying, Message::Result(_)) => Ok(()),
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Low-level send.
    pub async fn send_message(&mut self, msg: &Message) -> Result<(), ClientError> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1
            .send_msg_chunks(msg)
            .await
            .map_err(ClientError::Plexer)?;

        Ok(())
    }

    /// Low-level receive.
    pub async fn recv_message(&mut self) -> Result<Message, ClientError> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(ClientError::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    /// Send an `Acquire(point)` and transition to `Acquiring`.
    pub async fn send_acquire(&mut self, point: Option<Point>) -> Result<(), ClientError> {
        let msg = Message::Acquire(point);
        self.send_message(&msg).await?;
        self.0 = State::Acquiring;

        Ok(())
    }

    /// Drop the current snapshot and acquire `point` instead.
    pub async fn send_reacquire(&mut self, point: Option<Point>) -> Result<(), ClientError> {
        let msg = Message::ReAcquire(point);
        self.send_message(&msg).await?;
        self.0 = State::Acquiring;

        Ok(())
    }

    /// Release the current snapshot.
    pub async fn send_release(&mut self) -> Result<(), ClientError> {
        let msg = Message::Release;
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    /// Terminate the protocol.
    pub async fn send_done(&mut self) -> Result<(), ClientError> {
        let msg = Message::Done;
        self.send_message(&msg).await?;
        self.0 = State::Done;

        Ok(())
    }

    /// Wait for the server's `Acquired` (or `Failure`).
    pub async fn recv_while_acquiring(&mut self) -> Result<(), ClientError> {
        match self.recv_message().await? {
            Message::Acquired => {
                self.0 = State::Acquired;
                Ok(())
            }
            Message::Failure(x) => {
                self.0 = State::Idle;
                Err(ClientError::from(x))
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Acquire a snapshot at `point` (or the tip when `None`).
    pub async fn acquire(&mut self, point: Option<Point>) -> Result<(), ClientError> {
        self.send_acquire(point).await?;
        self.recv_while_acquiring().await
    }

    /// Send a `Query` carrying an arbitrary CBOR-encoded request.
    pub async fn send_query(&mut self, request: AnyCbor) -> Result<Message, ClientError> {
        let msg = Message::Query(request);
        self.send_message(&msg).await?;
        self.0 = State::Querying;

        Ok(msg)
    }

    /// Wait for the server's `Result` reply.
    pub async fn recv_while_querying(&mut self) -> Result<AnyCbor, ClientError> {
        match self.recv_message().await? {
            Message::Result(result) => {
                self.0 = State::Acquired;
                Ok(result)
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Run one untyped query, returning the raw CBOR result.
    pub async fn query_any(&mut self, request: AnyCbor) -> Result<AnyCbor, ClientError> {
        self.send_query(request).await?;
        self.recv_while_querying().await
    }

    /// Run one strongly-typed query, encoding the request and decoding the response.
    pub async fn query<Q, R>(&mut self, request: Q) -> Result<R, ClientError>
    where
        Q: pallas_codec::minicbor::Encode<()>,
        for<'b> R: pallas_codec::minicbor::Decode<'b, ()>,
    {
        let request = AnyCbor::from_encode(request);
        let response = self.query_any(request).await?;

        response.into_decode().map_err(ClientError::InvalidCbor)
    }
}

/// Concrete local-state-query client (default instantiation).
pub type Client = GenericClient;
