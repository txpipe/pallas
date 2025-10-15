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
        Self(State::Idle, multiplexer::ChannelBuffer::new(channel))
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    fn has_agency(&self) -> bool {
        matches!(&self.0, State::Busy)
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
        if self.state() != &State::Busy || matches!(msg, Message::RequestNext | Message::Done) {
            Err(ServerError::InvalidOutbound)
        } else {
            Ok(())
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), ServerError> {
        if self.state() == &State::Idle && matches!(msg, Message::RequestNext | Message::Done) {
            Ok(())
        } else {
            Err(ServerError::InvalidInbound)
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

    pub async fn recv_request_next(&mut self) -> Result<(), ServerError> {
        let msg = self.recv_message().await?;
        match msg {
            Message::RequestNext => {
                debug!("received Notification Request Next");
                self.0 = State::Busy;
                Ok(())
            }
            Message::Done => {
                debug!("client sent Done message in LeiosNotify protocol");
                self.0 = State::Done;
                Ok(())
            }
            _ => Err(ServerError::InvalidInbound),
        }
    }

    pub async fn send_block_announcement(
        &mut self,
        response: Header,
    ) -> Result<(), ServerError> {
        let msg = Message::BlockAnnouncement(response);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn send_block_offer(
        &mut self,
        slot: Slot,
        header: Header,
    ) -> Result<(), ServerError> {
        let msg = Message::BlockOffer(slot, header);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn send_block_txs_offer(
        &mut self,
        slot: Slot,
        header: Header,
    ) -> Result<(), ServerError> {
        let msg = Message::BlockTxsOffer(slot, header);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn send_vote_offer(
        &mut self,
        response: Vec<(Slot, VoteIssuerId)>,
    ) -> Result<(), ServerError> {
        let msg = Message::VoteOffer(response);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }
}
