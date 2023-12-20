use rand::Rng;
use std::fmt::Debug;
use thiserror::*;
use tracing::debug;

use super::protocol::*;
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

    #[error("keepalive cookie mismatch")]
    KeepAliveCookieMismatch,

    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

pub struct KeepAliveSharedState {
    saved_cookie: u16,
}

pub struct Client(State, multiplexer::ChannelBuffer, KeepAliveSharedState);

impl Client {
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(
            State::Client,
            multiplexer::ChannelBuffer::new(channel),
            KeepAliveSharedState { saved_cookie: 0 },
        )
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    fn has_agency(&self) -> bool {
        match &self.0 {
            State::Client => true,
            State::Server => false,
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

    fn assert_outbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Client, Message::KeepAlive(..)) => Ok(()),
            (State::Client, Message::Done) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Server, Message::ResponseKeepAlive(..)) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn send_message(&mut self, msg: &Message) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1.send_msg_chunks(msg).await.map_err(Error::Plexer)?;

        Ok(())
    }

    pub async fn recv_message(&mut self) -> Result<Message, Error> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(Error::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub async fn send_keepalive(&mut self) -> Result<(), Error> {
        // generate random cookie value
        let cookie = rand::thread_rng().gen::<KeepAliveCookie>();
        let msg = Message::KeepAlive(cookie);
        self.send_message(&msg).await?;
        self.2.saved_cookie = cookie;
        self.0 = State::Server;
        debug!("sent keepalive message with cookie {}", cookie);

        self.recv_while_sending_keepalive().await?;

        Ok(())
    }

    async fn recv_while_sending_keepalive(&mut self) -> Result<(), Error> {
        match self.recv_message().await? {
            Message::ResponseKeepAlive(cookie) => {
                debug!("received keepalive response with cookie {}", cookie);
                if cookie == self.2.saved_cookie {
                    self.0 = State::Client;
                    Ok(())
                } else {
                    Err(Error::KeepAliveCookieMismatch)
                }
            }
            _ => Err(Error::InvalidInbound),
        }
    }
}
