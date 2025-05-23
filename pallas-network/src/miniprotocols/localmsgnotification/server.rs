use pallas_codec::Fragment;

use crate::{miniprotocols::localmsgsubmission::DmqMsg, multiplexer};

use super::{protocol::Error, Message, State};

#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    NonBlocking,
    Blocking,
}

/// The DMQ server side of the local message notification protocol.
pub struct Server(State, multiplexer::ChannelBuffer)
where
    Message: Fragment;

impl Server
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
        !matches!(self.state(), State::Idle)
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

    /// As a server in a specific state, am I allowed to send this message?
    fn assert_outbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::BusyNonBlocking, Message::ReplyMessagesNonBlocking(..)) => Ok(()),
            (State::BusyBlocking, Message::ReplyMessagesBlocking(..)) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    /// As a server in a specific state, am I allowed to receive this message?
    fn assert_inbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Idle, Message::RequestMessagesNonBlocking) => Ok(()),
            (State::Idle, Message::RequestMessagesBlocking) => Ok(()),
            (State::Idle, Message::ClientDone) => Ok(()),
            _ => Err(Error::InvalidOutbound),
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

    pub async fn send_reply_messages_non_blocking(
        &mut self,
        msgs: Vec<DmqMsg>,
        has_more: bool,
    ) -> Result<(), Error> {
        let msg = Message::ReplyMessagesNonBlocking(msgs, has_more);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn send_reply_messages_blocking(&mut self, msgs: Vec<DmqMsg>) -> Result<(), Error> {
        let msg = Message::ReplyMessagesBlocking(msgs);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn recv_next_request(&mut self) -> Result<Request, Error> {
        match self.recv_message().await? {
            Message::RequestMessagesNonBlocking => {
                self.0 = State::BusyNonBlocking;

                Ok(Request::NonBlocking)
            }
            Message::RequestMessagesBlocking => {
                self.0 = State::BusyBlocking;

                Ok(Request::Blocking)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn recv_done(&mut self) -> Result<(), Error> {
        match self.recv_message().await? {
            Message::ClientDone => {
                self.0 = State::Done;

                Ok(())
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn send_done(&mut self) -> Result<(), Error> {
        let msg = Message::ServerDone;
        self.send_message(&msg).await?;
        self.0 = State::Done;

        Ok(())
    }
}
