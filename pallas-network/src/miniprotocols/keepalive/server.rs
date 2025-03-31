use std::fmt::Debug;
use thiserror::*;
use tracing::debug;

use super::protocol::*;
use crate::{
    miniprotocols::{Agent, Error, PlexerAdapter},
    multiplexer,
};

pub struct Server(State);

impl Default for Server {
    fn default() -> Self {
        Self(State::Client(ClientState::Empty))
    }
}

impl Agent for Server {
    type State = State;
    type Message = Message;

    fn new(init: Self::State) -> Self {
        Self(init)
    }

    fn is_done(&self) -> bool {
        matches!(self.0, State::Done)
    }

    fn has_agency(&self) -> bool {
        match self.state() {
            State::Client(..) => false,
            State::Server(..) => true,
            State::Done => false,
        }
    }

    fn state(&self) -> &Self::State {
        &self.0
    }

    fn apply(&self, msg: &Self::Message) -> Result<Self::State, Error> {
        match self.state() {
            State::Client(..) => match msg {
                Message::KeepAlive(_) => todo!(),
                _ => Err(Error::InvalidInbound),
            },
            State::Server(..) => match msg {
                Message::ResponseKeepAlive(_) => todo!(),
                _ => Err(Error::InvalidOutbound),
            },
            State::Done => Err(Error::InvalidInbound),
        }
    }
}

impl PlexerAdapter<Server> {
    pub async fn recv_keepalive_request(&mut self) -> Result<(), Error> {
        self.recv().await?;

        match self.state() {
            State::Server(x) => {
                debug!("received keepalive message with cookie {}", x);
                Ok(())
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn send_keepalive_response(&mut self) -> Result<(), Error> {
        match self.state() {
            State::Server(x) => {
                let msg = Message::ResponseKeepAlive(*x);
                self.send(&msg).await
            }
            _ => Err(Error::InvalidOutbound),
        }
    }

    pub async fn keepalive_roundtrip(&mut self) -> Result<(), Error> {
        self.recv_keepalive_request().await?;
        self.send_keepalive_response().await?;

        Ok(())
    }
}
