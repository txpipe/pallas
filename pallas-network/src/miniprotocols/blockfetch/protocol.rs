use crate::miniprotocols::Point;

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
