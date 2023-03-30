use pallas_codec::Fragment;
use pallas_multiplexer::agents::{Channel, ChannelBuffer, ChannelError};
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::common::Point;

use super::{Message, State};

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    #[error("attempted to send message while agency is theirs")]
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

pub type Range = (Point, Point);

pub type HasBlocks = Option<()>;

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

    pub fn recv_while_busy(&mut self) -> Result<HasBlocks, Error> {
        match self.recv_message()? {
            Message::StartBatch => {
                info!("batch start");
                self.0 = State::Streaming;
                Ok(Some(()))
            }
            Message::NoBlocks => {
                warn!("no blocks");
                self.0 = State::Idle;
                Ok(None)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn request_range(&mut self, range: Range) -> Result<HasBlocks, Error> {
        self.send_request_range(range)?;
        debug!("range requested");
        self.recv_while_busy()
    }

    pub fn recv_while_streaming(&mut self) -> Result<Option<Body>, Error> {
        match self.recv_message()? {
            Message::Block { body } => Ok(Some(body)),
            Message::BatchDone => {
                self.0 = State::Idle;
                Ok(None)
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub fn fetch_single(&mut self, point: Point) -> Result<Body, Error> {
        self.request_range((point.clone(), point))?
            .ok_or(Error::NoBlocks)?;

        let body = self.recv_while_streaming()?.ok_or(Error::InvalidInbound)?;
        debug!("body received");

        match self.recv_while_streaming()? {
            Some(_) => Err(Error::InvalidInbound),
            None => Ok(body),
        }
    }

    pub fn fetch_range(&mut self, range: Range) -> Result<Vec<Body>, Error> {
        self.request_range(range)?.ok_or(Error::NoBlocks)?;

        let mut all = vec![];

        while let Some(block) = self.recv_while_streaming()? {
            debug!("body received");
            all.push(block);
        }

        Ok(all)
    }

    pub fn send_done(&mut self) -> Result<(), Error> {
        let msg = Message::ClientDone;
        self.send_message(&msg)?;
        self.0 = State::Done;

        Ok(())
    }
}
