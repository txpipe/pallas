use pallas_codec::Fragment;
use pallas_multiplexer::agents::{Channel, ChannelBuffer, ChannelError};
use thiserror::Error;

use crate::common::Point;

use super::{Message, State};

#[derive(Error, Debug)]
pub enum Error {
    #[error("attemted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attemted to send message while agency is theirs")]
    AgencyIsTheirs,

    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    #[error("requested range doesn't contain any blocks")]
    NoBlocks,

    #[error("error while sending or receiving data through the channel")]
    ChannelError(ChannelError),
}

pub type Body = Vec<u8>;

pub struct Client<H>(State, ChannelBuffer<H>)
where
    H: Channel,
    Message: Fragment;

impl<H> Client<H>
where
    H: Channel,
    Message: Fragment,
{
    pub fn new(channel: H) -> Self {
        Self(State::Idle, ChannelBuffer::new(channel))
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => true,
            State::Busy => false,
            State::Streaming => false,
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
            (State::Idle, Message::RequestRange { .. }) => Ok(()),
            (State::Idle, Message::ClientDone) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Busy, Message::StartBatch) => Ok(()),
            (State::Busy, Message::NoBlocks) => Ok(()),
            (State::Streaming, Message::Block { .. }) => Ok(()),
            (State::Streaming, Message::BatchDone) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn send_message(&mut self, msg: &Message) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1.send_msg_chunks(msg).map_err(Error::ChannelError)?;

        Ok(())
    }

    pub fn recv_message(&mut self) -> Result<Message, Error> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().map_err(Error::ChannelError)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub fn send_request_range(&mut self, range: (Point, Point)) -> Result<(), Error> {
        let msg = Message::RequestRange { range };
        self.send_message(&msg)?;
        self.0 = State::Busy;

        Ok(())
    }

    pub fn recv_request_range(&mut self) -> Result<(), Error> {
        match self.recv_message()? {
            Message::StartBatch => {
                self.0 = State::Streaming;
                Ok(())
            }
            Message::NoBlocks => {
                self.0 = State::Idle;
                Err(Error::NoBlocks)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn request_range(&mut self, range: (Point, Point)) -> Result<(), Error> {
        self.send_request_range(range)?;
        self.recv_request_range()
    }

    pub fn recv_next_block(&mut self) -> Result<Option<Body>, Error> {
        match self.recv_message()? {
            Message::Block { body } => {
                self.0 = State::Streaming;
                Ok(Some(body))
            }
            Message::BatchDone => {
                self.0 = State::Idle;
                Ok(None)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn send_done(&mut self) -> Result<(), Error> {
        let msg = Message::ClientDone;
        self.send_message(&msg)?;
        self.0 = State::Done;

        Ok(())
    }
}
