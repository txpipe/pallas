use pallas_codec::utils::TagWrap;

/// Absolute slot number used to tag a mempool snapshot.
pub type Slot = u64;
/// Transaction id rendered as a string (hex of the tx hash).
pub type TxId = String;
/// Era number, as carried in the multi-era transaction wrapper.
pub type Era = u8;
/// Raw CBOR bytes of a transaction body.
pub type TxBody = pallas_codec::utils::Bytes;
/// `(era, cbor-tag-24-wrapped body)` — the canonical mempool transaction shape.
pub type Tx = (Era, TagWrap<TxBody, 24>);

/// Tx-monitor state-machine state.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Idle, no snapshot acquired.
    Idle,
    /// Awaiting acquisition of a mempool snapshot.
    Acquiring,
    /// Snapshot acquired; ready for queries.
    Acquired,
    /// Server is computing a response.
    Busy,
    /// Protocol terminated.
    Done,
}

/// Mempool size accounting reported by `ResponseSizeAndCapacity`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MempoolSizeAndCapacity {
    /// Maximum total transaction bytes the mempool will hold.
    pub capacity_in_bytes: u32,
    /// Total bytes currently held in the mempool.
    pub size_in_bytes: u32,
    /// Number of transactions currently in the mempool.
    pub number_of_txs: u32,
}

/// Tx-monitor protocol message.
#[derive(Debug, Clone)]
pub enum Message {
    /// Client → server: acquire the current mempool snapshot (non-blocking).
    Acquire,
    /// Client → server: acquire the next snapshot (blocks until it changes).
    AwaitAcquire,
    /// Server → client: snapshot acquired at the given slot.
    Acquired(Slot),
    /// Client → server: ask whether a specific transaction is in the snapshot.
    RequestHasTx(TxId),
    /// Client → server: iterate to the next transaction in the snapshot.
    RequestNextTx,
    /// Client → server: ask for mempool size and capacity.
    RequestSizeAndCapacity,
    /// Server → client: answer to [`Message::RequestHasTx`].
    ResponseHasTx(bool),
    /// Server → client: next transaction (or `None` if the iteration is exhausted).
    ResponseNextTx(Option<Tx>),
    /// Server → client: answer to [`Message::RequestSizeAndCapacity`].
    ResponseSizeAndCapacity(MempoolSizeAndCapacity),
    /// Client → server: release the current snapshot.
    Release,
    /// Client → server: terminate the protocol.
    Done,
}
