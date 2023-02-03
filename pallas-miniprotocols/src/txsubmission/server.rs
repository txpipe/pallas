use pallas_codec::Fragment;
use pallas_multiplexer::agents::{Channel, ChannelBuffer};

use super::protocol::{Message, State, TxBody, TxId, TxIdAndSize, Error, Blocking, TxCount};

pub enum Reply {
    TxIds(Vec<TxIdAndSize>),
    Txs(Vec<TxBody>),
    Done,
}

pub struct Server<H>(State, ChannelBuffer<H>)
where
    H: Channel,
    Message: Fragment;

impl<H> Server<H>
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

    // NOTE(pi): as of this writing, the network spec has a typo; this is the correct behavior
    fn has_agency(&self) -> bool {
        match self.state() {
            State::Idle => true,
            _ => false,
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

    /// As a server in a specific state, am I allowed to send this message?
    fn assert_outbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Idle, Message::RequestTxIds(..)) => Ok(()),
            (State::Idle, Message::RequestTxs(..)) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    /// As a server in a specific state, am I allowed to receive this message?
    fn assert_inbound_state(&self, msg: &Message) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Init, Message::Init) => Ok(()),
            (State::TxIdsBlocking, Message::ReplyTxIds(..)) => Ok(()),
            (State::TxIdsBlocking, Message::Done) => Ok(()),
            (State::TxIdsNonBlocking, Message::ReplyTxIds(..)) => Ok(()),
            (State::Txs, Message::ReplyTxs(..)) => Ok(()),
            _ => Err(Error::InvalidOutbound),
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

    pub fn wait_for_init(&mut self) -> Result<(), Error> {
        if self.0 != State::Init {
            return Err(Error::AlreadyInitialized);
        }
        
        // recv_message calls assert_inbound_state, which ensures we get an init message
        self.recv_message()?;
        self.0 = State::Idle;

        Ok(())
    }

    pub fn acknowledge_and_request_tx_ids(&mut self, blocking: Blocking, acknowledge: TxCount, count: TxCount) -> Result<(), Error> {
        let msg = Message::RequestTxIds(blocking, acknowledge, count);
        self.send_message(&msg)?;
        match blocking {
            true => self.0 = State::TxIdsBlocking,
            false => self.0 = State::TxIdsNonBlocking,
        }

        Ok(())
    }

    pub fn request_txs(&mut self, ids: Vec<TxId>) -> Result<(), Error> {
        let msg = Message::RequestTxs(ids);
        self.send_message(&msg)?;
        self.0 = State::Txs;

        Ok(())
    }

    pub fn receive_next_reply(&mut self) -> Result<Reply, Error> {
        match self.recv_message()? {
            Message::ReplyTxIds(ids_and_sizes) => {
                self.0 = State::Idle;
                
                Ok(Reply::TxIds(ids_and_sizes))
            }
            Message::ReplyTxs(bodies) => {
                self.0 = State::Txs;
                Ok(Reply::Txs(bodies))
            }
            Message::Done => {
                self.0 = State::Done;
                Ok(Reply::Done)
            }
            _ => Err(Error::InvalidInbound),
        }
    }
}
