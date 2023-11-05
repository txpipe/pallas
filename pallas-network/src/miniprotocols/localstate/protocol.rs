use std::convert::Into;
use std::fmt::Debug;

use crate::miniprotocols::Point;

use super::queries::{Request, Response};

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
    type Request: Clone + Debug + Into<Request>;
    type Response: Clone + Debug + Into<Response>;

    fn to_vec(response: Self::Response) -> Vec<u8>;
    fn map_response(signal: u16, response: Vec<u8>) -> Self::Response;
    fn request_signal(request: Self::Request) -> u16;
}

#[derive(Debug)]
pub enum Message<Q: Query> {
    Acquire(Option<Point>),
    Failure(AcquireFailure),
    Acquired,
    Query(Q::Request),
    Response(Q::Response),
    ReAcquire(Option<Point>),
    Release,
    Done,
}
