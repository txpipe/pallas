use crate::miniprotocols::Point;

pub type Body = Vec<u8>;

pub type Range = (Point, Point);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ClientState {
    Idle,
    Busy,
    Streaming(Option<Body>),
    Done,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ServerState {
    Idle,
    Busy(Range),
    Streaming,
    Done,
}

#[derive(Debug, Clone)]
pub enum Message {
    RequestRange(Range),
    ClientDone,
    StartBatch,
    NoBlocks,
    Block(Body),
    BatchDone,
}
