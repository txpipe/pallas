use std::fmt::Debug;
use thiserror::*;
use tracing::debug;

use super::protocol::*;
use crate::multiplexer;

/// Errors produced by the peer-sharing client agent.
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

    /// Server returned a different number of peers than requested.
    #[error("requested amount mismatch")]
    RequestedAmountMismatch,

    /// Underlying multiplexer error.
    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

/// Peer-sharing client agent.
pub struct Client(State, multiplexer::ChannelBuffer);

impl Client {
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

    /// True if the client holds agency.
    pub fn has_agency(&self) -> bool {
        match &self.0 {
            State::Idle => true,
            State::Busy(..) => false,
            State::Done => false,
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
            (State::Idle, Message::ShareRequest(..)) => Ok(()),
            (State::Idle, Message::Done) => Ok(()),
            _ => Err(ClientError::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), ClientError> {
        match (&self.0, msg) {
            (State::Busy(..), Message::SharePeers(..)) => Ok(()),
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

    /// Ask the peer to share up to `amount` known peer addresses.
    pub async fn send_share_request(&mut self, amount: Amount) -> Result<(), ClientError> {
        let msg = Message::ShareRequest(amount);
        self.send_message(&msg).await?;
        self.0 = State::Busy(amount);
        debug!(amount, "sent share request message");

        Ok(())
    }

    /// Wait for the peer's `SharePeers` reply.
    pub async fn recv_peer_addresses(&mut self) -> Result<Vec<PeerAddress>, ClientError> {
        let msg = self.recv_message().await?;
        match msg {
            Message::SharePeers(addresses) => {
                debug!(
                    length = addresses.len(),
                    ?addresses,
                    "received peer addresses"
                );
                self.0 = State::Idle;
                Ok(addresses)
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Terminate the protocol.
    pub async fn send_done(&mut self) -> Result<(), ClientError> {
        let msg = Message::Done;
        self.send_message(&msg).await?;
        self.0 = State::Done;

        Ok(())
    }
}
