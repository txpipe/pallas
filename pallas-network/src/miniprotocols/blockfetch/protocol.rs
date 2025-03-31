use crate::miniprotocols::Point;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    Busy,
    Streaming,
    Done,
}

#[derive(Debug, Clone)]
pub enum Message {
    RequestRange { range: (Point, Point) },
    ClientDone,
    StartBatch,
    NoBlocks,
    Block { body: Vec<u8> },
    BatchDone,
}
