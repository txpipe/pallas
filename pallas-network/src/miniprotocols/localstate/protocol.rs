use std::fmt::Debug;

use pallas_codec::utils::AnyCbor;

use crate::miniprotocols::Point;

/// Local-state-query state-machine state.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Idle: client may acquire a snapshot or terminate.
    Idle,
    /// Awaiting confirmation of an acquire attempt.
    Acquiring,
    /// A snapshot is acquired; client may query, re-acquire, or release.
    Acquired,
    /// Awaiting the result of a query.
    Querying,
    /// Protocol terminated.
    Done,
}

/// Reason a snapshot could not be acquired.
#[derive(Debug)]
pub enum AcquireFailure {
    /// The requested point is too old to be available.
    PointTooOld,
    /// The requested point is not on the current chain.
    PointNotOnChain,
}

/// Local-state-query protocol message.
#[derive(Debug)]
pub enum Message {
    /// Client → server: acquire a snapshot at `Point` (or the tip when `None`).
    Acquire(Option<Point>),
    /// Server → client: acquire failed for the stated reason.
    Failure(AcquireFailure),
    /// Server → client: snapshot acquired.
    Acquired,
    /// Client → server: run a query against the current snapshot.
    Query(AnyCbor),
    /// Server → client: result of the previous query.
    Result(AnyCbor),
    /// Client → server: drop the current snapshot and acquire a new one.
    ReAcquire(Option<Point>),
    /// Client → server: release the current snapshot.
    Release,
    /// Client → server: terminate the protocol.
    Done,
}
