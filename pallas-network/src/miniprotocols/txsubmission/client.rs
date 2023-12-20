use std::marker::PhantomData;

use crate::multiplexer;
use pallas_codec::Fragment;

use super::{
    protocol::{Error, Message, State, TxIdAndSize},
    EraTxBody, EraTxId,
};

pub enum Request<TxId> {
    TxIds(u16, u16),
    TxIdsNonBlocking(u16, u16),
    Txs(Vec<TxId>),
}

/// A generic Ouroboros client for submitting a generic notion of "transactions"
/// to another server
pub struct GenericClient<TxId, TxBody>(
    State,
    multiplexer::ChannelBuffer,
    PhantomData<TxId>,
    PhantomData<TxBody>,
)
where
    Message<TxId, TxBody>: Fragment;

/// A cardano specific instantiation of the ouroboros protocol
pub type Client = GenericClient<EraTxId, EraTxBody>;

impl<TxId, TxBody> GenericClient<TxId, TxBody>
where
    Message<TxId, TxBody>: Fragment,
{
    pub fn new(channel: multiplexer::AgentChannel) -> Self {
        Self(
            State::Init,
            multiplexer::ChannelBuffer::new(channel),
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

    /// As a client in a specific state, am I allowed to send this message?
    fn assert_outbound_state(&self, msg: &Message<TxId, TxBody>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Init, Message::Init) => Ok(()),
            (State::TxIdsBlocking, Message::ReplyTxIds(..)) => Ok(()),
            (State::TxIdsBlocking, Message::Done) => Ok(()),
            (State::TxIdsNonBlocking, Message::ReplyTxIds(..)) => Ok(()),
            (State::Txs, Message::ReplyTxs(..)) => Ok(()),
            _ => Err(Error::InvalidOutbound),
        }
    }

    /// As a client in a specific state, am I allowed to receive this message?
    fn assert_inbound_state(&self, msg: &Message<TxId, TxBody>) -> Result<(), Error> {
        match (&self.0, msg) {
            (State::Idle, Message::RequestTxIds(..)) => Ok(()),
            (State::Idle, Message::RequestTxs(..)) => Ok(()),
            _ => Err(Error::InvalidInbound),
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

    pub async fn send_init(&mut self) -> Result<(), Error> {
        let msg = Message::Init;
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn reply_tx_ids(&mut self, ids: Vec<TxIdAndSize<TxId>>) -> Result<(), Error> {
        let msg = Message::ReplyTxIds(ids);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn reply_txs(&mut self, txs: Vec<TxBody>) -> Result<(), Error> {
        let msg = Message::ReplyTxs(txs);
        self.send_message(&msg).await?;
        self.0 = State::Idle;

        Ok(())
    }

    pub async fn next_request(&mut self) -> Result<Request<TxId>, Error> {
        match self.recv_message().await? {
            Message::RequestTxIds(blocking, ack, req) => match blocking {
                true => {
                    self.0 = State::TxIdsBlocking;
                    Ok(Request::TxIds(ack, req))
                }
                false => {
                    self.0 = State::TxIdsNonBlocking;
                    Ok(Request::TxIdsNonBlocking(ack, req))
                }
            },
            Message::RequestTxs(x) => {
                self.0 = State::Txs;
                Ok(Request::Txs(x))
            }
            _ => Err(Error::InvalidInbound),
        }
    }

    pub async fn send_done(&mut self) -> Result<(), Error> {
        let msg = Message::Done;
        self.send_message(&msg).await?;
        self.0 = State::Done;

        Ok(())
    }
}
