use rand::Rng;
use tracing::debug;

use super::protocol::*;
use crate::{
    miniprotocols::{Agent, Error, PlexerAdapter},
    multiplexer,
};

#[derive(Debug)]
pub struct Client(State);

impl Default for Client {
    fn default() -> Self {
        Self(State::Client(ClientState::Empty))
    }
}

impl Agent for Client {
    type State = State;
    type Message = Message;

    fn new(init: Self::State) -> Self {
        Self(init)
    }

    fn is_done(&self) -> bool {
        matches!(self.0, State::Done)
    }

    fn has_agency(&self) -> bool {
        match &self.0 {
            State::Client(..) => true,
            State::Server(..) => false,
            State::Done => true,
        }
    }

    fn state(&self) -> &Self::State {
        &self.0
    }

    fn apply(&self, msg: &Self::Message) -> Result<Self::State, Error> {
        match self.state() {
            State::Client(..) => match msg {
                Message::KeepAlive(x) => Ok(State::Server(*x)),
                _ => Err(Error::InvalidOutbound),
            },
            State::Server(x) => match msg {
                Message::ResponseKeepAlive(x) => Ok(State::Client(ClientState::Response(*x))),
                _ => Err(Error::InvalidInbound),
            },
            State::Done => Err(Error::InvalidOutbound),
        }
    }
}

impl PlexerAdapter<Client> {
    pub async fn send_keepalive_request(&mut self) -> Result<(), Error> {
        // generate random cookie value
        let cookie = rand::thread_rng().gen::<Cookie>();
        let msg = Message::KeepAlive(cookie);

        self.send(&msg).await?;

        debug!("sent keepalive message with cookie {}", cookie);

        Ok(())
    }

    pub async fn recv_keepalive_response(&mut self) -> Result<(), Error> {
        let expected = match self.state() {
            State::Server(x) => *x,
            _ => return Err(Error::InvalidInbound),
        };

        self.recv().await?;

        if let State::Client(ClientState::Response(received)) = self.state() {
            if *received == expected {
                return Ok(());
            }
        }

        Err(Error::InvalidInbound)
    }

    pub async fn keepalive_roundtrip(&mut self) -> Result<(), Error> {
        self.send_keepalive_request().await?;
        self.recv_keepalive_response().await?;

        Ok(())
    }
}
