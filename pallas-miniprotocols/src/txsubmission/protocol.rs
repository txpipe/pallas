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

// The bytes of a txId, tagged with an era number
#[derive(Debug, Clone)]
pub struct EraTxId(pub u16, pub Vec<u8>);

#[derive(Debug)]
pub struct TxIdAndSize<TxID>(pub TxID, pub TxSizeInBytes);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxBody(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct Tx<TxId>(pub TxId, pub TxBody);

impl<TxId> From<Tx<TxId>> for TxIdAndSize<TxId> {
    fn from(other: Tx<TxId>) -> Self {
        TxIdAndSize(other.0, other.1 .0.len() as u32)
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
pub enum Message<TxId> {
    Init,
    RequestTxIds(Blocking, TxCount, TxCount),
    ReplyTxIds(Vec<TxIdAndSize<TxId>>),
    RequestTxs(Vec<TxId>),
    ReplyTxs(Vec<TxBody>),
    Done,
}
