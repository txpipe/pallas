use std::fmt::Debug;
use tracing::debug;

use super::protocol::*;
use crate::miniprotocols::{Agent, Error, PlexerAdapter};

#[derive(Debug)]
pub struct Client(State);

impl Default for Client {
    fn default() -> Self {
        Self(State::Idle(IdleState::Empty))
    }
}

impl Agent for Client {
    type Message = Message;
    type State = State;

    fn new(init: Self::State) -> Self {
        Self(init)
    }

    fn is_done(&self) -> bool {
        matches!(self.0, State::Done)
    }

    fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle(..) => true,
            State::Busy(..) => false,
            State::Done => true,
        }
    }

    fn state(&self) -> &Self::State {
        &self.0
    }

    fn apply(&self, msg: &Self::Message) -> Result<Self::State, Error> {
        match self.state() {
            State::Idle(..) => match msg {
                Message::ShareRequest(x) => Ok(State::Busy(*x)),
                _ => Err(Error::InvalidOutbound),
            },
            State::Busy(..) => match msg {
                Message::SharePeers(x) => Ok(State::Idle(IdleState::Response(x.clone()))),
                _ => Err(Error::InvalidInbound),
            },
            State::Done => Err(Error::InvalidOutbound),
        }
    }
}

impl PlexerAdapter<Client> {
    pub async fn send_share_request(&mut self, amount: Amount) -> Result<(), Error> {
        let msg = Message::ShareRequest(amount);
        self.send(&msg).await?;

        debug!(amount, "sent share request message");

        Ok(())
    }

    pub async fn recv_peer_addresses(&mut self) -> Result<Vec<PeerAddress>, Error> {
        self.recv().await?;

        match self.state() {
            State::Idle(IdleState::Response(addresses)) => Ok(addresses.clone()),
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn send_done(&mut self) -> Result<(), Error> {
        let msg = Message::Done;

        self.send(&msg).await?;

        Ok(())
    }
}
