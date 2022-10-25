use pallas_codec::Fragment;
use pallas_multiplexer::agents::{Channel, ChannelBuffer, ChannelError};
use std::marker::PhantomData;
use thiserror::*;

use super::{Message, RefuseReason, State, VersionNumber, VersionTable};

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
    ChannelError(ChannelError),
}

#[derive(Debug)]
pub enum Confirmation<D> {
    Accepted(VersionNumber, D),
    Rejected(RefuseReason),
}

pub struct Client<H, D>(State, ChannelBuffer<H>, PhantomData<D>)
where
    H: Channel;

impl<H, D> Client<H, D>
where
    H: Channel,
    D: std::fmt::Debug + Clone,
    Message<D>: Fragment,
{
    pub fn new(channel: H) -> Self {
        Self(State::Propose, ChannelBuffer::new(channel), PhantomData {})
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    pub fn has_agency(&self) -> bool {
        match self.state() {
            State::Propose => true,
            State::Confirm => false,
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

    fn assert_outbound_state(&self, msg: &Message<D>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Propose, Message::Propose(_)) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message<D>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Confirm, Message::Accept(..)) => Ok(()),
            (State::Confirm, Message::Refuse(..)) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn send_message(&mut self, msg: &Message<D>) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1.send_msg_chunks(msg).map_err(Error::ChannelError)?;

        Ok(())
    }

    pub fn recv_message(&mut self) -> Result<Message<D>, Error> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().map_err(Error::ChannelError)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub fn send_propose(&mut self, versions: VersionTable<D>) -> Result<(), Error> {
        let msg = Message::Propose(versions);
        self.send_message(&msg)?;
        self.0 = State::Confirm;

        Ok(())
    }

    pub fn recv_while_confirm(&mut self) -> Result<Confirmation<D>, Error> {
        match self.recv_message()? {
            Message::Accept(v, m) => {
                self.0 = State::Done;
                Ok(Confirmation::Accepted(v, m))
            }
            Message::Refuse(r) => {
                self.0 = State::Done;
                Ok(Confirmation::Rejected(r))
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn handshake(&mut self, versions: VersionTable<D>) -> Result<Confirmation<D>, Error> {
        self.send_propose(versions)?;
        self.recv_while_confirm()
    }
}

pub type N2NClient<H> = Client<H, super::n2n::VersionData>;

pub type N2CClient<H> = Client<H, super::n2c::VersionData>;
