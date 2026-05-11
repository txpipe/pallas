use std::fmt::Debug;
use thiserror::*;
use tracing::debug;

use super::protocol::*;
use crate::multiplexer;

/// Errors produced by the peer-sharing server agent.
#[derive(Error, Debug)]
pub enum ServerError {
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

/// Peer-sharing server agent.
pub struct Server(State, multiplexer::ChannelBuffer);

impl Server {
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
        match &self.0 {
            State::Idle => false,
            State::Busy(..) => true,
            State::Done => false,
        }
    }

    fn assert_agency_is_ours(&self) -> Result<(), ServerError> {
        if !self.has_agency() {
            Err(ServerError::AgencyIsTheirs)
        } else {
            Ok(())
        }
    }

    fn assert_agency_is_theirs(&self) -> Result<(), ServerError> {
        if self.has_agency() {
            Err(ServerError::AgencyIsOurs)
        } else {
            Ok(())
        }
    }

    fn assert_outbound_state(&self, msg: &Message) -> Result<(), ServerError> {
        match (&self.0, msg) {
            (State::Busy(..), Message::SharePeers(..)) => Ok(()),
            _ => Err(ServerError::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), ServerError> {
        match (&self.0, msg) {
            (State::Idle, Message::ShareRequest(..)) => Ok(()),
            (State::Idle, Message::Done) => Ok(()),
            _ => Err(ServerError::InvalidInbound),
        }
    }

    /// Low-level send.
    pub async fn send_message(&mut self, msg: &Message) -> Result<(), ServerError> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1
            .send_msg_chunks(msg)
            .await
            .map_err(ServerError::Plexer)?;

        Ok(())
    }

    /// Low-level receive.
    pub async fn recv_message(&mut self) -> Result<Message, ServerError> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(ServerError::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    /// Wait for the next `ShareRequest` (or `Done`) and update state.
    pub async fn recv_share_request(&mut self) -> Result<Option<Amount>, ServerError> {
        let msg = self.recv_message().await?;
        match msg {
            Message::ShareRequest(amount) => {
                debug!(amount, "received share request");
                self.0 = State::Busy(amount);
                Ok(Some(amount))
            }
            Message::Done => {
                debug!("client sent done message in peersharing protocol");
                self.0 = State::Done;
                Ok(None)
            }
            _ => Err(ServerError::InvalidInbound),
        }
    }

    /// Reply to the pending share request with the given peer addresses.
    pub async fn send_peer_addresses(
        &mut self,
        response: Vec<PeerAddress>,
    ) -> Result<(), ServerError> {
        let msg = Message::SharePeers(response);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }
}
