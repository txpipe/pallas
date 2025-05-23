use pallas_codec::Fragment;

use crate::{miniprotocols::localmsgsubmission::DmqMsg, multiplexer};

use super::{protocol::Error, Message, State};

#[derive(Debug, PartialEq, Eq)]
pub struct Reply(pub Vec<DmqMsg>, pub bool);

/// The DMQ client side of the local message notification protocol.
pub struct Client(State, multiplexer::ChannelBuffer)
where
    Message: Fragment;

impl Client
where
    Message: Fragment,
{
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
        matches!(self.state(), State::Idle)
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

    /// As a client in a specific state, am I allowed to send this message?
    fn assert_outbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Idle, Message::RequestMessagesNonBlocking) => Ok(()),
            (State::Idle, Message::RequestMessagesBlocking) => Ok(()),
            (State::Idle, Message::ClientDone) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    /// As a client in a specific state, am I allowed to receive this message?
    fn assert_inbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::BusyNonBlocking, Message::ReplyMessagesNonBlocking(..)) => Ok(()),
            (State::BusyBlocking, Message::ReplyMessagesBlocking(..)) => Ok(()),
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

    pub async fn send_request_messages_non_blocking(&mut self) -> Result<(), Error> {
        let msg = Message::RequestMessagesNonBlocking;
        self.send_message(&msg).await?;
        self.0 = State::BusyNonBlocking;

        Ok(())
    }

    pub async fn send_request_messages_blocking(&mut self) -> Result<(), Error> {
        let msg = Message::RequestMessagesBlocking;
        self.send_message(&msg).await?;
        self.0 = State::BusyBlocking;

        Ok(())
    }

    pub async fn recv_next_reply(&mut self) -> Result<Reply, Error> {
        match self.recv_message().await? {
            Message::ReplyMessagesNonBlocking(msgs, has_more) => {
                self.0 = State::Idle;

                Ok(Reply(msgs, has_more))
            }
            Message::ReplyMessagesBlocking(msgs) => {
                self.0 = State::Idle;

                Ok(Reply(msgs, false))
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn recv_done(&mut self) -> Result<(), Error> {
        match self.recv_message().await? {
            Message::ServerDone => {
                self.0 = State::Done;

                Ok(())
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn send_done(&mut self) -> Result<(), Error> {
        let msg = Message::ClientDone;
        self.send_message(&msg).await?;
        self.0 = State::Done;

        Ok(())
    }
}
