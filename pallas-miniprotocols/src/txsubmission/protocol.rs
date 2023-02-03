use pallas_multiplexer::agents::ChannelError;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Init,
    Idle,
    TxIdsNonBlocking,
    TxIdsBlocking,
    Txs,
    Done,
}

pub type Blocking = bool;

pub type TxCount = u16;

pub type TxSizeInBytes = u32;

pub type TxId = Vec<u8>;

#[derive(Debug)]
pub struct TxIdAndSize(pub TxId, pub TxSizeInBytes);

pub type TxBody = Vec<u8>;

#[derive(Debug, Clone)]
pub struct Tx(pub TxId, pub TxBody);

impl From<Tx> for TxIdAndSize {
    fn from(other: Tx) -> Self {
        TxIdAndSize(other.0, other.1.len() as u32)
    }
}

impl From<TxIdAndSize> for TxId {
    fn from(value: TxIdAndSize) -> Self {
        value.0
    }
}


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

    #[error("protocol is already initialized, no need to wait for init message")]
    AlreadyInitialized,

    #[error("error while sending or receiving data through the channel")]
    ChannelError(ChannelError),
}


#[derive(Debug)]
pub enum Message {
    Init,
    RequestTxIds(Blocking, TxCount, TxCount),
    ReplyTxIds(Vec<TxIdAndSize>),
    RequestTxs(Vec<TxId>),
    ReplyTxs(Vec<TxBody>),
    Done,
}