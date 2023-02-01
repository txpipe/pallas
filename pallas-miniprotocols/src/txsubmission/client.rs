use pallas_codec::Fragment;
use pallas_multiplexer::agents::{Channel, ChannelBuffer, ChannelError};
use thiserror::Error;

use super::protocol::{Message, State, Tx, TxId};

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

pub enum Request {
    TxIds(u32),
    TxIdsNoneBlocking(u32),
    Txs(Vec<TxId>),
}

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
        Self(State::Init, ChannelBuffer::new(channel))
    }

    pub fn state(&self) -> &State {
        &self.0
    }

    pub fn is_done(&self) -> bool {
        self.0 == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => false,
            _ => true,
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
            (State::Init, Message::Init) => Ok(()),
            (State::TxIdsBlocking, Message::ReplyTxIds(..)) => Ok(()),
            (State::TxIdsNonBlocking, Message::ReplyTxIds(..)) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    fn assert_inbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Idle, Message::RequestTxIds(..)) => Ok(()),
            (State::Idle, Message::RequestTxIdsNonBlocking(..)) => Ok(()),
            (State::Idle, Message::RequestTxs(..)) => Ok(()),
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

    pub fn send_init(&mut self) -> Result<(), Error> {
        let msg = Message::Init;
        self.send_message(&msg)?;
        self.0 = State::Idle;

        Ok(())
    }

    pub fn reply_tx_ids(&mut self, ids: Vec<TxId>) -> Result<(), Error> {
        let msg = Message::ReplyTxIds(ids);
        self.send_message(&msg)?;
        self.0 = State::Idle;

        Ok(())
    }

    pub fn reply_txs(&mut self, txs: Vec<Tx>) -> Result<(), Error> {
        let msg = Message::ReplyTxs(txs);
        self.send_message(&msg)?;
        self.0 = State::Idle;

        Ok(())
    }

    pub fn next_request(&mut self) -> Result<Message, Error> {
        match self.recv_message()? {
            Message::RequestTxIds(x) => {
                self.0 = State::TxIdsBlocking;
                Ok(Request::TxIds(x))
            }
            Message::RequestTxIdsNonBlocking(x) => {
                self.0 = State::TxIdsNonBlocking;
                Ok(Request::TxIdsNoneBlocking(x))
            }
            Message::ReplyTxs(x) => {
                self.0 = State::Txs;
                Ok(Request::Txs(x))
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
