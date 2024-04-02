use std::fmt::Debug;
use thiserror::*;
use tracing::debug;

use super::protocol::*;
use crate::multiplexer;

#[derive(Error, Debug)]
pub enum ServerError {
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

pub struct Server(State, multiplexer::ChannelBuffer);

impl Server {
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(State::Client, multiplexer::ChannelBuffer::new(channel))
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    fn has_agency(&self) -> bool {
        match &self.0 {
            State::Client => false,
            State::Server(..) => true,
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
            (State::Server(..), Message::ResponseKeepAlive(..)) => Ok(()),
            _ => Err(ServerError::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), ServerError> {
        match (&self.0, msg) {
            (State::Client, Message::KeepAlive(..)) => Ok(()),
            (State::Client, Message::Done) => Ok(()),
            _ => Err(ServerError::InvalidInbound),
        }
    }

    pub async fn send_message(&mut self, msg: &Message) -> Result<(), ServerError> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1
            .send_msg_chunks(msg)
            .await
            .map_err(ServerError::Plexer)?;

        Ok(())
    }

    pub async fn recv_message(&mut self) -> Result<Message, ServerError> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(ServerError::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub async fn recv_keepalive_request(&mut self) -> Result<(), ServerError> {
        match self.recv_message().await? {
            Message::KeepAlive(cookie) => {
                debug!("received keepalive message with cookie {}", cookie);
                self.0 = State::Server(cookie);
                Ok(())
            }
            Message::Done => {
                debug!("client sent done message in keepalive protocol");
                self.0 = State::Done;
                Ok(())
            }
            _ => Err(ServerError::InvalidInbound),
        }
    }

    pub async fn send_keepalive_response(&mut self) -> Result<(), ServerError> {
        match self.state().clone() {
            State::Server(cookie) => {
                let msg = Message::ResponseKeepAlive(cookie);
                self.send_message(&msg).await?;
                self.0 = State::Client;
                debug!("sent keepalive response message with cookie {}", cookie);
            }
            _ => (),
        }

        Ok(())
    }

    pub async fn keepalive_roundtrip(&mut self) -> Result<(), ServerError> {
        self.recv_keepalive_request().await?;
        self.send_keepalive_response().await?;

        Ok(())
    }
}
