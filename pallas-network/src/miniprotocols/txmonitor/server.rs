use std::fmt::Debug;
use thiserror::*;

use super::protocol::*;
use crate::multiplexer;

/// Errors produced by the tx-monitor server agent.
#[derive(Error, Debug)]
pub enum Error {
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

/// Request received from the client while a snapshot is acquired.
#[derive(Debug)]
pub enum ClientQueryRequest {
    /// Drop the current snapshot and acquire the next one (blocking until the
    /// mempool changes).
    AwaitAcquire,
    /// Iterate to the next transaction in the snapshot.
    NextTx,
    /// Ask whether a specific transaction is in the snapshot.
    HasTx(TxId),
    /// Ask for mempool size and capacity.
    GetSizes,
    /// Ask for mempool measures (node-to-client v20+).
    GetMeasures,
    /// Release the current snapshot.
    Release,
}

/// Tx-monitor server agent.
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
        matches!(self.state(), State::Acquiring | State::Busy)
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
            (State::Acquiring, Message::Acquired(..)) => Ok(()),
            (State::Busy, Message::ResponseNextTx(..)) => Ok(()),
            (State::Busy, Message::ResponseHasTx(..)) => Ok(()),
            (State::Busy, Message::ResponseSizeAndCapacity(..)) => Ok(()),
            (State::Busy, Message::ResponseGetMeasures(..)) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Idle, Message::Acquire) => Ok(()),
            (State::Idle, Message::Done) => Ok(()),
            // wire label 1 in the acquired state means await re-acquire
            (State::Acquired, Message::Acquire | Message::AwaitAcquire) => Ok(()),
            (State::Acquired, Message::RequestNextTx) => Ok(()),
            (State::Acquired, Message::RequestHasTx(..)) => Ok(()),
            (State::Acquired, Message::RequestSizeAndCapacity) => Ok(()),
            (State::Acquired, Message::RequestGetMeasures) => Ok(()),
            (State::Acquired, Message::Release) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    /// Low-level send.
    pub async fn send_message(&mut self, msg: &Message) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1.send_msg_chunks(msg).await.map_err(Error::Plexer)?;

        Ok(())
    }

    /// Low-level receive.
    pub async fn recv_message(&mut self) -> Result<Message, Error> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(Error::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    /// Confirm the pending acquire, tagging the snapshot with the given slot.
    pub async fn send_acquired(&mut self, slot: Slot) -> Result<(), Error> {
        let msg = Message::Acquired(slot);
        self.send_message(&msg).await?;
        self.0 = State::Acquired;

        Ok(())
    }

    /// Reply to the pending [`ClientQueryRequest::NextTx`] request.
    pub async fn send_next_tx(&mut self, tx: Option<Tx>) -> Result<(), Error> {
        let msg = Message::ResponseNextTx(tx);
        self.send_message(&msg).await?;
        self.0 = State::Acquired;

        Ok(())
    }

    /// Reply to the pending [`ClientQueryRequest::HasTx`] request.
    pub async fn send_has_tx(&mut self, has: bool) -> Result<(), Error> {
        let msg = Message::ResponseHasTx(has);
        self.send_message(&msg).await?;
        self.0 = State::Acquired;

        Ok(())
    }

    /// Reply to the pending [`ClientQueryRequest::GetSizes`] request.
    pub async fn send_size_and_capacity(
        &mut self,
        sizes: MempoolSizeAndCapacity,
    ) -> Result<(), Error> {
        let msg = Message::ResponseSizeAndCapacity(sizes);
        self.send_message(&msg).await?;
        self.0 = State::Acquired;

        Ok(())
    }

    /// Reply to the pending [`ClientQueryRequest::GetMeasures`] request.
    pub async fn send_measures(&mut self, measures: MempoolMeasures) -> Result<(), Error> {
        let msg = Message::ResponseGetMeasures(measures);
        self.send_message(&msg).await?;
        self.0 = State::Acquired;

        Ok(())
    }

    /// Wait for the next request while the protocol is in the `Idle` state.
    /// Returns `None` if the client terminated the protocol.
    pub async fn recv_while_idle(&mut self) -> Result<Option<()>, Error> {
        match self.recv_message().await? {
            Message::Acquire => {
                self.0 = State::Acquiring;
                Ok(Some(()))
            }
            Message::Done => {
                self.0 = State::Done;
                Ok(None)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    /// Wait for the next request while a snapshot is acquired.
    pub async fn recv_while_acquired(&mut self) -> Result<ClientQueryRequest, Error> {
        match self.recv_message().await? {
            Message::Acquire | Message::AwaitAcquire => {
                self.0 = State::Acquiring;
                Ok(ClientQueryRequest::AwaitAcquire)
            }
            Message::RequestNextTx => {
                self.0 = State::Busy;
                Ok(ClientQueryRequest::NextTx)
            }
            Message::RequestHasTx(id) => {
                self.0 = State::Busy;
                Ok(ClientQueryRequest::HasTx(id))
            }
            Message::RequestSizeAndCapacity => {
                self.0 = State::Busy;
                Ok(ClientQueryRequest::GetSizes)
            }
            Message::RequestGetMeasures => {
                self.0 = State::Busy;
                Ok(ClientQueryRequest::GetMeasures)
            }
            Message::Release => {
                self.0 = State::Idle;
                Ok(ClientQueryRequest::Release)
            }
            _ => Err(Error::InvalidInbound),
        }
    }
}
