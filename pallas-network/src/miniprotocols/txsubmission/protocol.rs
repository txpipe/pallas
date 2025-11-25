use thiserror::Error;

use crate::multiplexer;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EraTxId(pub u16, pub Vec<u8>);

// The bytes of a transaction, with an era number and some raw CBOR
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EraTxBody(pub u16, pub Vec<u8>);

#[derive(Debug, PartialEq, Eq)]
pub struct TxIdAndSize<TxID>(pub TxID, pub TxSizeInBytes);

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
    Plexer(multiplexer::Error),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Message<TxId, TxBody> {
    Init,
    RequestTxIds(Blocking, TxCount, TxCount),
    ReplyTxIds(Vec<TxIdAndSize<TxId>>),
    RequestTxs(Vec<TxId>),
    ReplyTxs(Vec<TxBody>),
    Done,
}
