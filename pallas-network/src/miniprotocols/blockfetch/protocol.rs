use crate::miniprotocols::Point;

/// Largest inbound message the agents accept in [`State::Idle`], in bytes, as
/// in the Ouroboros network specification.
pub const IDLE_SIZE_LIMIT: usize = 65_535;

/// Largest inbound message the agents accept in [`State::Busy`], in bytes.
pub const BUSY_SIZE_LIMIT: usize = 65_535;

/// Largest inbound message the agents accept in [`State::Streaming`], in
/// bytes, which bounds the size of a block.
pub const STREAMING_SIZE_LIMIT: usize = 2_500_000;

/// Block-fetch state machine state.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Idle: client may request a range or terminate.
    Idle,
    /// Server is deciding whether to serve the requested range.
    Busy,
    /// Server is streaming `Block` messages.
    Streaming,
    /// Protocol is terminated.
    Done,
}

impl State {
    pub(crate) fn size_limit(&self) -> usize {
        match self {
            State::Idle | State::Done => IDLE_SIZE_LIMIT,
            State::Busy => BUSY_SIZE_LIMIT,
            State::Streaming => STREAMING_SIZE_LIMIT,
        }
    }
}

/// Block-fetch protocol message.
#[derive(Debug)]
pub enum Message {
    /// Client → server: request all blocks in the inclusive range `[start, end]`.
    RequestRange {
        /// Inclusive `(start, end)` chain points.
        range: (Point, Point),
    },
    /// Client → server: terminate the protocol.
    ClientDone,
    /// Server → client: range is available; start streaming.
    StartBatch,
    /// Server → client: range cannot be served.
    NoBlocks,
    /// Server → client: one block body in the streamed batch.
    Block {
        /// Raw CBOR bytes of the block body.
        body: Vec<u8>,
    },
    /// Server → client: end of the current batch.
    BatchDone,
}
