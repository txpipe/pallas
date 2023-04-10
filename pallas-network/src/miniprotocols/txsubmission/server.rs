use std::marker::PhantomData;

use pallas_codec::Fragment;

use super::{
    protocol::{Blocking, Error, Message, State, TxCount, TxIdAndSize},
    EraTxBody, EraTxId,
};
use crate::plexer;

pub enum Reply<TxId, TxBody> {
    TxIds(Vec<TxIdAndSize<TxId>>),
    Txs(Vec<TxBody>),
    Done,
}

/// A generic implementation of an ouroboros server protocol ready to request
/// and receive transactions from a client
pub struct GenericServer<TxId, TxBody>(
    State,
    plexer::ChannelBuffer,
    PhantomData<TxId>,
    PhantomData<TxBody>,
)
where
    Message<TxId, TxBody>: Fragment;

/// A Cardano specific server for the ouroboros TxSubmission protocol
pub type Server = GenericServer<EraTxId, EraTxBody>;

impl<TxId, TxBody> GenericServer<TxId, TxBody>
where
    Message<TxId, TxBody>: Fragment,
{
    pub fn new(channel: plexer::AgentChannel) -> Self {
        Self(
            State::Init,
            plexer::ChannelBuffer::new(channel),
            PhantomData {},
            PhantomData {},
        )
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

    /// As a server in a specific state, am I allowed to send this message?
    fn assert_outbound_state(&self, msg: &Message<TxId, TxBody>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Idle, Message::RequestTxIds(..)) => Ok(()),
            (State::Idle, Message::RequestTxs(..)) => Ok(()),
            _ => Err(Error::InvalidInbound),
        }
    }

    /// As a server in a specific state, am I allowed to receive this message?
    fn assert_inbound_state(&self, msg: &Message<TxId, TxBody>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Init, Message::Init) => Ok(()),
            (State::TxIdsBlocking, Message::ReplyTxIds(..)) => Ok(()),
            (State::TxIdsBlocking, Message::Done) => Ok(()),
            (State::TxIdsNonBlocking, Message::ReplyTxIds(..)) => Ok(()),
            (State::Txs, Message::ReplyTxs(..)) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    pub async fn send_message(&mut self, msg: &Message<TxId, TxBody>) -> Result<(), Error> {
        self.assert_agency_is_ours()?;
        self.assert_outbound_state(msg)?;
        self.1.send_msg_chunks(msg).await.map_err(Error::Plexer)?;

        Ok(())
    }

    pub async fn recv_message(&mut self) -> Result<Message<TxId, TxBody>, Error> {
        self.assert_agency_is_theirs()?;
        let msg = self.1.recv_full_msg().await.map_err(Error::Plexer)?;
        self.assert_inbound_state(&msg)?;

        Ok(msg)
    }

    pub async fn wait_for_init(&mut self) -> Result<(), Error> {
        if self.0 != State::Init {
            return Err(Error::AlreadyInitialized);
        }

        // recv_message calls assert_inbound_state, which ensures we get an init message
        self.recv_message().await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn acknowledge_and_request_tx_ids(
        &mut self,
        blocking: Blocking,
        acknowledge: TxCount,
        count: TxCount,
    ) -> Result<(), Error> {
        let msg = Message::RequestTxIds(blocking, acknowledge, count);
        self.send_message(&msg).await?;
        match blocking {
            true => self.0 = State::TxIdsBlocking,
            false => self.0 = State::TxIdsNonBlocking,
        }

        Ok(())
    }

    pub async fn request_txs(&mut self, ids: Vec<TxId>) -> Result<(), Error> {
        let msg = Message::RequestTxs(ids);
        self.send_message(&msg).await?;
        self.0 = State::Txs;

        Ok(())
    }

    pub async fn receive_next_reply(&mut self) -> Result<Reply<TxId, TxBody>, Error> {
        match self.recv_message().await? {
            Message::ReplyTxIds(ids_and_sizes) => {
                self.0 = State::Idle;

                Ok(Reply::TxIds(ids_and_sizes))
            }
            Message::ReplyTxs(bodies) => {
                self.0 = State::Idle;
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
