use pallas_codec::utils::TagWrap;

/// Absolute slot number used to tag a mempool snapshot.
pub type Slot = u64;
/// Era number, as carried in the multi-era transaction wrapper.
pub type Era = u8;
/// Raw bytes of a transaction hash.
pub type TxIdBytes = pallas_codec::utils::Bytes;
/// `(era, tx-hash-bytes)` — era-wrapped transaction id, mirroring the
/// hard-fork-combinator `GenTxId` encoding used by node-to-client peers.
///
/// Note that peers (e.g. cardano-node, ogmios) treat the era wrapper as
/// significant for equality: the same hash wrapped in different eras is a
/// different `GenTxId`. Clients typically probe every plausible era.
pub type TxId = (Era, TxIdBytes);
/// Raw CBOR bytes of a transaction body.
pub type TxBody = pallas_codec::utils::Bytes;
/// `(era, cbor-tag-24-wrapped body)` — the canonical mempool transaction shape.
pub type Tx = (Era, TagWrap<TxBody, 24>);
/// Name of a mempool measure reported by `ResponseGetMeasures`.
pub type MeasureName = String;

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

/// Current size and maximum capacity of a single mempool measure.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SizeAndCapacity {
    /// Current size of the measure.
    pub size: u64,
    /// Maximum capacity of the measure.
    pub capacity: u64,
}

/// Mempool measures reported by `ResponseGetMeasures` (node-to-client v20+).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MempoolMeasures {
    /// Number of transactions currently in the mempool.
    pub tx_count: u32,
    /// Named measures (e.g. transaction bytes, execution units) with their
    /// current size and capacity.
    pub measures: Vec<(MeasureName, SizeAndCapacity)>,
}

/// Tx-monitor protocol message.
#[derive(Debug, Clone)]
pub enum Message {
    /// Client → server: acquire the current mempool snapshot (non-blocking).
    Acquire,
    /// Client → server: acquire the next snapshot (blocks until it changes).
    ///
    /// On the wire this shares label `1` with [`Message::Acquire`]; peers
    /// disambiguate by protocol state (`Idle` → acquire, `Acquired` → await
    /// re-acquire). A stateless decode therefore always yields
    /// [`Message::Acquire`]; agents map it back based on their state.
    AwaitAcquire,
    /// Server → client: snapshot acquired at the given slot.
    Acquired(Slot),
    /// Client → server: ask whether a specific transaction is in the snapshot.
    RequestHasTx(TxId),
    /// Client → server: iterate to the next transaction in the snapshot.
    RequestNextTx,
    /// Client → server: ask for mempool size and capacity.
    RequestSizeAndCapacity,
    /// Client → server: ask for mempool measures (node-to-client v20+).
    RequestGetMeasures,
    /// Server → client: answer to [`Message::RequestHasTx`].
    ResponseHasTx(bool),
    /// Server → client: next transaction (or `None` if the iteration is exhausted).
    ResponseNextTx(Option<Tx>),
    /// Server → client: answer to [`Message::RequestSizeAndCapacity`].
    ResponseSizeAndCapacity(MempoolSizeAndCapacity),
    /// Server → client: answer to [`Message::RequestGetMeasures`].
    ResponseGetMeasures(MempoolMeasures),
    /// Client → server: release the current snapshot.
    Release,
    /// Client → server: terminate the protocol.
    Done,
}
