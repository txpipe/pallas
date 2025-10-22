use std::fmt::Debug;
use thiserror::*;
use tracing::debug;

use super::protocol::*;
use crate::{
    miniprotocols::leiosnotify::{Hash, Slot, VoteIssuerId},
    multiplexer,
};

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("attempted to send message after protocol is done")]
    ProtocolDone,

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

    /// Returns the current state of the client.
    pub fn state(&self) -> &State {
        &self.0
    }

    /// Checks if the client is done.
    pub fn is_done(&self) -> bool {
        self.state() == &State::Done
    }

    /// Checks if the client has agency.
    fn has_agency(&self) -> bool {
        self.state() == &State::Idle
    }

    fn assert_agency_is_ours(&self) -> Result<(), ClientError> {
        if self.is_done() {
            Err(ClientError::ProtocolDone)
        } else if !self.has_agency() {
            Err(ClientError::AgencyIsTheirs)
        } else {
            Ok(())
        }
    }

    fn assert_agency_is_theirs(&self) -> Result<(), ClientError> {
        if self.has_agency() {
            Err(ClientError::AgencyIsOurs)
        } else if self.is_done() {
            Err(ClientError::ProtocolDone)
        } else {
            Ok(())
        }
    }

    fn assert_outbound_state(&self, msg: &Message) -> Result<(), ClientError> {
        use Message::*;

        if self.state() == &State::Idle
            && matches!(
                msg,
                BlockRequest(..) | BlockTxsRequest(..) | VoteRequest(..) | RangeRequest { .. }
            )
        {
            Ok(())
        } else {
            Err(ClientError::InvalidOutbound)
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), ClientError> {
        use Message::*;

        match (self.state(), msg) {
            (State::Block, Block(_)) => Ok(()),
            (State::BlockTxs, BlockTxs(_)) => Ok(()),
            (State::Votes, VoteDelivery(..)) => Ok(()),
            (State::BlockRange, NextBlockAndTxs(..)) => Ok(()),
            (State::BlockRange, LastBlockAndTxs(..)) => Ok(()),
            _ => Err(ClientError::InvalidOutbound),
        }
    }

    pub async fn send_message(&mut self, msg: &Message) -> Result<(), ClientError> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1
            .send_msg_chunks(msg)
            .await
            .map_err(ClientError::Plexer)?;

        Ok(())
    }

    pub async fn recv_message(&mut self) -> Result<Message, ClientError> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(ClientError::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub async fn send_block_request(&mut self, slot: Slot, hash: Hash) -> Result<(), ClientError> {
        let msg = Message::BlockRequest(slot, hash);
        self.send_message(&msg).await?;
        self.0 = State::Block;
        debug!("sent block request");

        Ok(())
    }

    pub async fn send_block_txs_request(
        &mut self,
        slot: Slot,
        hash: Hash,
        tx_map: TxMap,
    ) -> Result<(), ClientError> {
        let msg = Message::BlockTxsRequest(slot, hash, tx_map);
        self.send_message(&msg).await?;
        self.0 = State::BlockTxs;
        debug!("sent block and txs request");

        Ok(())
    }

    pub async fn send_vote_request(
        &mut self,
        req: Vec<(Slot, VoteIssuerId)>,
    ) -> Result<(), ClientError> {
        let msg = Message::VoteRequest(req);
        self.send_message(&msg).await?;
        self.0 = State::Votes;
        debug!("sent vote request");

        Ok(())
    }

    pub async fn send_range_request(
        &mut self,
        first: (Slot, Hash),
        last: (Slot, Hash),
    ) -> Result<(), ClientError> {
        let msg = Message::RangeRequest { first, last };
        self.send_message(&msg).await?;
        self.0 = State::BlockRange;
        debug!("sent vote request");

        Ok(())
    }

    pub async fn send_done(&mut self) -> Result<(), ClientError> {
        let msg = Message::Done;
        self.send_message(&msg).await?;
        self.0 = State::Done;

        Ok(())
    }
    pub async fn recv_block(&mut self) -> Result<EndorserBlock, ClientError> {
        let msg = self.recv_message().await?;
        match msg {
            Message::Block(block) => {
                self.0 = State::Idle;
                Ok(block)
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    pub async fn recv_block_txs(&mut self) -> Result<Vec<AnyCbor>, ClientError> {
        let msg = self.recv_message().await?;
        match msg {
            Message::BlockTxs(response) => {
                tracing::trace!(?response, "received");
                self.0 = State::Idle;
                Ok(response)
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    pub async fn recv_vote_delivery(&mut self) -> Result<Vec<LeiosVote>, ClientError> {
        let msg = self.recv_message().await?;
        match msg {
            Message::VoteDelivery(votes) => {
                tracing::trace!(?votes, "received");
                self.0 = State::Idle;
                Ok(votes)
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    pub async fn recv_while_block_range(
        &mut self,
    ) -> Result<(EndorserBlock, Vec<AnyCbor>), ClientError> {
        match self.recv_message().await? {
            Message::NextBlockAndTxs(block, txs) => {
                debug!("Receiving next block and txs");
                tracing::trace!(?block, ?txs, "received");
                self.0 = State::Idle;
                Ok((block, txs))
            }
            Message::LastBlockAndTxs(block, txs) => {
                debug!("Receiving last block and txs");
                tracing::trace!(?block, ?txs, "received");
                self.0 = State::Idle;
                Ok((block, txs))
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }
}
