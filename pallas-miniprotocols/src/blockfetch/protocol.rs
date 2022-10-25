use crate::Point;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    Busy,
    Streaming,
    Done,
}

#[derive(Debug)]
pub enum Message {
    RequestRange { range: (Point, Point) },
    ClientDone,
    StartBatch,
    NoBlocks,
    Block { body: Vec<u8> },
    BatchDone,
}
