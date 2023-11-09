use std::convert::Into;
use std::fmt::Debug;

use pallas_codec::utils::AnyCbor;

use crate::miniprotocols::Point;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    Acquiring,
    Acquired,
    Querying,
    Done,
}

#[derive(Debug)]
pub enum AcquireFailure {
    PointTooOld,
    PointNotOnChain,
}

#[derive(Debug)]
pub enum Message {
    Acquire(Option<Point>),
    Failure(AcquireFailure),
    Acquired,
    Query(AnyCbor),
    Response(AnyCbor),
    ReAcquire(Option<Point>),
    Release,
    Done,
}
