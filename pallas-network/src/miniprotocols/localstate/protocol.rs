use std::fmt::Debug;

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

pub trait Query: Debug {
    type Request: Clone + Debug;
    type Response: Clone + Debug;
}

#[derive(Debug)]
pub enum Message<Q: Query> {
    Acquire(Option<Point>),
    Failure(AcquireFailure),
    Acquired,
    Query(Q::Request),
    Result(Q::Response),
    ReAcquire(Option<Point>),
    Release,
    Done,
}
