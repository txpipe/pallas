use thiserror::Error;

use crate::multiplexer;

/// Largest inbound message the agents accept in [`State::Init`], in bytes, as
/// in the Ouroboros network specification.
pub const INIT_SIZE_LIMIT: usize = 65_535;

/// Largest inbound message the agents accept in [`State::Idle`], in bytes.
pub const IDLE_SIZE_LIMIT: usize = 65_535;

/// Largest inbound message the agents accept in [`State::TxIdsBlocking`] and
/// [`State::TxIdsNonBlocking`], in bytes.
pub const TX_IDS_SIZE_LIMIT: usize = 2_500_000;

/// Largest inbound message the agents accept in [`State::Txs`], in bytes.
pub const TXS_SIZE_LIMIT: usize = 2_500_000;

/// Tx-submission state-machine state.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Initial state, awaiting the server's `Init` message.
    Init,
    /// Idle: server may request tx ids or transactions.
    Idle,
    /// Awaiting a non-blocking tx-id reply.
    TxIdsNonBlocking,
    /// Awaiting a blocking tx-id reply (server waits for new txs to arrive).
    TxIdsBlocking,
    /// Awaiting transaction bodies for previously announced ids.
    Txs,
    /// Protocol terminated.
    Done,
}

impl State {
    pub(crate) fn size_limit(&self) -> usize {
        match self {
            State::Init => INIT_SIZE_LIMIT,
            State::Idle | State::Done => IDLE_SIZE_LIMIT,
            State::TxIdsBlocking | State::TxIdsNonBlocking => TX_IDS_SIZE_LIMIT,
            State::Txs => TXS_SIZE_LIMIT,
        }
    }
}

/// Whether a tx-id request should block until new txs are available.
pub type Blocking = bool;

/// Number of transactions requested or returned.
pub type TxCount = u16;

/// Transaction size in bytes.
pub type TxSizeInBytes = u32;

/// The bytes of a tx-id, tagged with the era number it was produced in.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EraTxId(pub u16, pub Vec<u8>);

/// The bytes of a transaction body, tagged with the era number it was produced in.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EraTxBody(pub u16, pub Vec<u8>);

/// A transaction id paired with the size (in bytes) of its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxIdAndSize<TxID>(pub TxID, pub TxSizeInBytes);

/// Errors produced by the tx-submission protocol.
#[derive(Error, Debug)]
pub enum Error {
    /// Tried to receive while we hold agency.
    #[error("attempted to receive message while agency is ours")]
    AgencyIsOurs,

    /// Tried to send while the peer holds agency.
    #[error("attempted to send message while agency is theirs")]
    AgencyIsTheirs,

    /// Inbound message is not valid for the current state.
    #[error("inbound message is not valid for current state")]
    InvalidInbound,

    /// Outbound message is not valid for the current state.
    #[error("outbound message is not valid for current state")]
    InvalidOutbound,

    /// Caller tried to re-initialize an already-running protocol.
    #[error("protocol is already initialized, no need to wait for init message")]
    AlreadyInitialized,

    /// Underlying multiplexer error.
    #[error("error while sending or receiving data through the channel")]
    Plexer(multiplexer::Error),
}

/// Tx-submission protocol message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message<TxId, TxBody> {
    /// Server → client: open the protocol.
    Init,
    /// Server → client: request tx ids — `blocking`, `ack` (previously acknowledged), `req` (max new to return).
    RequestTxIds(Blocking, TxCount, TxCount),
    /// Client → server: reply with available tx ids and their sizes.
    ReplyTxIds(Vec<TxIdAndSize<TxId>>),
    /// Server → client: request full bodies for previously announced ids.
    RequestTxs(Vec<TxId>),
    /// Client → server: reply with the requested transaction bodies.
    ReplyTxs(Vec<TxBody>),
    /// Client → server: terminate the protocol.
    Done,
}
