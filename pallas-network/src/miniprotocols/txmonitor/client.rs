use std::fmt::Debug;
use thiserror::*;

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

    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

pub struct Client(State, multiplexer::ChannelBuffer);

impl Client {
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(State::Idle, multiplexer::ChannelBuffer::new(channel))
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    fn has_agency(&self) -> bool {
        match &self.0 {
            State::Idle => true,
            State::Acquiring => false,
            State::Acquired => true,
            State::Busy => false,
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
            (State::Idle, Message::Acquire) => Ok(()),
            (State::Idle, Message::Done) => Ok(()),
            (State::Acquired, Message::Acquire) => Ok(()),
            (State::Acquired, Message::RequestHasTx(..)) => Ok(()),
            (State::Acquired, Message::RequestNextTx) => Ok(()),
            (State::Acquired, Message::RequestSizeAndCapacity) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Acquiring, Message::Acquired(..)) => Ok(()),
            (State::Busy, Message::ResponseHasTx(..)) => Ok(()),
            (State::Busy, Message::ResponseNextTx(..)) => Ok(()),
            (State::Busy, Message::ResponseSizeAndCapacity(..)) => Ok(()),
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

    async fn send_acquire(&mut self) -> Result<(), Error> {
        let msg = Message::Acquire;
        self.send_message(&msg).await?;
        self.0 = State::Acquiring;

        Ok(())
    }

    async fn recv_while_acquiring(&mut self) -> Result<Slot, Error> {
        match self.recv_message().await? {
            Message::Acquired(slot) => {
                self.0 = State::Acquired;
                Ok(slot)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn acquire(&mut self) -> Result<Slot, Error> {
        self.send_acquire().await?;
        self.recv_while_acquiring().await
    }

    async fn send_request_has_tx(&mut self, id: TxId) -> Result<(), Error> {
        let msg = Message::RequestHasTx(id);
        self.send_message(&msg).await?;
        self.0 = State::Busy;

        Ok(())
    }

    async fn recv_while_requesting_has_tx(&mut self) -> Result<bool, Error> {
        match self.recv_message().await? {
            Message::ResponseHasTx(x) => {
                self.0 = State::Acquired;
                Ok(x)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn query_has_tx(&mut self, id: TxId) -> Result<bool, Error> {
        self.send_request_has_tx(id).await?;
        self.recv_while_requesting_has_tx().await
    }

    async fn send_request_next_tx(&mut self) -> Result<(), Error> {
        let msg = Message::RequestNextTx;
        self.send_message(&msg).await?;
        self.0 = State::Busy;

        Ok(())
    }

    async fn recv_while_requesting_next_tx(&mut self) -> Result<Option<Tx>, Error> {
        match self.recv_message().await? {
            Message::ResponseNextTx(x) => {
                self.0 = State::Acquired;
                Ok(x)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn query_next_tx(&mut self) -> Result<Option<Tx>, Error> {
        self.send_request_next_tx().await?;
        self.recv_while_requesting_next_tx().await
    }

    async fn send_request_size_and_capacity(&mut self) -> Result<(), Error> {
        let msg = Message::RequestSizeAndCapacity;
        self.send_message(&msg).await?;
        self.0 = State::Busy;

        Ok(())
    }

    async fn recv_while_requesting_size_and_capacity(
        &mut self,
    ) -> Result<MempoolSizeAndCapacity, Error> {
        match self.recv_message().await? {
            Message::ResponseSizeAndCapacity(x) => {
                self.0 = State::Acquired;
                Ok(x)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn query_size_and_capacity(&mut self) -> Result<MempoolSizeAndCapacity, Error> {
        self.send_request_size_and_capacity().await?;
        self.recv_while_requesting_size_and_capacity().await
    }

    pub async fn release(&mut self) -> Result<(), Error> {
        let msg = Message::Release;
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }
}
