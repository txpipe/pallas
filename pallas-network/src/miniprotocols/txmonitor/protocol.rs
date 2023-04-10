pub type Slot = u64;
pub type TxId = String;
pub type Tx = Vec<u8>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    Acquiring,
    Acquired,
    Busy,
    Done,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MempoolSizeAndCapacity {
    pub capacity_in_bytes: u32,
    pub size_in_bytes: u32,
    pub number_of_txs: u32,
}

#[derive(Debug, Clone)]
pub enum Message {
    Acquire,
    AwaitAcquire,
    Acquired(Slot),
    RequestHasTx(TxId),
    RequestNextTx,
    RequestSizeAndCapacity,
    ResponseHasTx(bool),
    ResponseNextTx(Option<Tx>),
    ResponseSizeAndCapacity(MempoolSizeAndCapacity),
    Release,
    Done,
}
