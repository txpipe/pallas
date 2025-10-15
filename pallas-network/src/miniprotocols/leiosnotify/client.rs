use std::fmt::Debug;
use thiserror::*;
use tracing::debug;

use super::protocol::*;
use crate::multiplexer;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    // #[error("")]
    // ProtocolSpecificError,

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
        if self.state() == &State::Idle && matches!(msg, Message::RequestNext | Message::Done) {
            Ok(())
        } else {
            Err(ClientError::InvalidOutbound)
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), ClientError> {
        if self.state() != &State::Busy || matches!(msg, Message::RequestNext | Message::Done) {
            Err(ClientError::InvalidInbound)
        } else {
            Ok(())
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

    pub async fn send_request_next(&mut self) -> Result<(), ClientError> {
        let msg = Message::RequestNext;
        self.send_message(&msg).await?;
        self.0 = State::Busy;
        debug!("sent notification request next message");

        Ok(())
    }

    pub async fn recv_block_announcement(&mut self) -> Result<Header, ClientError> {
        let msg = self.recv_message().await?;
        match msg {
            Message::BlockAnnouncement(params) => {
                debug!(
                    ?params,
                    "received "
                );
                self.0 = State::Idle;
                Ok(params)
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    pub async fn recv_block_offer(&mut self) -> Result<BlockOffer, ClientError> {
        let msg = self.recv_message().await?;
        match msg {
            Message::BlockOffer(slot, hash) => {
                debug!(
                    ?slot,
                    ?hash,
                    "received "
                );
                self.0 = State::Idle;
                Ok((slot, hash))
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    pub async fn recv_block_tx_offer(&mut self) -> Result<BlockTxsOffer, ClientError> {
        let msg = self.recv_message().await?;
        match msg {
            Message::BlockTxsOffer(slot, hash) => {
                debug!(
                    ?slot,
                    ?hash,
                    "received "
                );
                self.0 = State::Idle;
                Ok((slot, hash))
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    pub async fn recv_vote_offer(&mut self) -> Result<Vec<(Slot, VoteIssuerId)>, ClientError> {
        let msg = self.recv_message().await?;
        match msg {
            Message::VoteOffer(params) => {
                debug!(
                    ?params,
                    "received "
                );
                self.0 = State::Idle;
                Ok(params)
            }
            _ => Err(ClientError::InvalidInbound),
        }
    }

    pub async fn send_done(&mut self) -> Result<(), ClientError> {
        let msg = Message::Done;
        self.send_message(&msg).await?;
        self.0 = State::Done;

        Ok(())
    }
}
