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

impl From<&Tx> for TxIdAndSize {
    fn from(other: &Tx) -> Self {
        TxIdAndSize(other.0, other.1.len() as u32)
    }
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
