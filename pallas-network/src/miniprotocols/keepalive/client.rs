use rand::RngExt;
use std::fmt::Debug;
use thiserror::*;
use tracing::debug;

use super::protocol::*;
use crate::multiplexer;

/// Errors produced by the keep-alive client agent.
#[derive(Error, Debug)]
pub enum ClientError {
    /// Tried to receive while the client holds agency.
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

    /// Server echoed a cookie that does not match the one we sent.
    #[error("keepalive cookie mismatch")]
    KeepAliveCookieMismatch,

    /// Underlying multiplexer error.
    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

/// Keep-alive client agent.
pub struct Client(State, multiplexer::ChannelBuffer);

impl Client {
    /// Build a client over a freshly subscribed agent channel.
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(State::Client, multiplexer::ChannelBuffer::new(channel))
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
            State::Client => true,
            State::Server(..) => false,
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
            (State::Client, Message::KeepAlive(..)) => Ok(()),
            (State::Client, Message::Done) => Ok(()),
            _ => Err(ClientError::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), ClientError> {
        match (&self.0, msg) {
            (State::Server(..), Message::ResponseKeepAlive(..)) => Ok(()),
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Low-level send. Use [`Self::keepalive_roundtrip`] for the common case.
    pub async fn send_message(&mut self, msg: &Message) -> Result<(), ClientError> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1
            .send_msg_chunks(msg)
            .await
            .map_err(ClientError::Plexer)?;

        Ok(())
    }

    /// Low-level receive. Use [`Self::keepalive_roundtrip`] for the common case.
    pub async fn recv_message(&mut self) -> Result<Message, ClientError> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(ClientError::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    /// Send a `KeepAlive` request carrying a freshly generated cookie.
    pub async fn send_keepalive_request(&mut self) -> Result<(), ClientError> {
        // generate random cookie value
        let cookie = rand::rng().random::<Cookie>();
        let msg = Message::KeepAlive(cookie);
        self.send_message(&msg).await?;
        self.0 = State::Server(cookie);
        debug!("sent keepalive message with cookie {}", cookie);

        Ok(())
    }

    /// Receive the matching `ResponseKeepAlive` and verify the cookie.
    pub async fn recv_keepalive_response(&mut self) -> Result<(), ClientError> {
        match self.recv_message().await? {
            Message::ResponseKeepAlive(cookie) => {
                debug!("received keepalive response with cookie {}", cookie);
                match self.state() {
                    State::Server(expected) if *expected == cookie => {
                        self.0 = State::Client;
                        Ok(())
                    }
                    State::Server(..) => Err(ClientError::KeepAliveCookieMismatch),
                    _ => unreachable!(),
                }
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    /// Send a ping and wait for its matching response.
    pub async fn keepalive_roundtrip(&mut self) -> Result<(), ClientError> {
        self.send_keepalive_request().await?;
        self.recv_keepalive_response().await?;

        Ok(())
    }
}
