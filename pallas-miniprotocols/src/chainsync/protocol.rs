use std::fmt::Debug;

use crate::common::Point;
use crate::machines::{DecodePayload, EncodePayload};

#[derive(Debug)]
pub struct Tip(pub Point, pub u64);

#[derive(Debug, PartialEq, Clone)]
pub enum State {
    Idle,
    CanAwait,
    MustReply,
    Intersect,
    Done,
}

/// A generic chain-sync message for either header or block content
#[derive(Debug)]
pub enum Message<C>
where
    C: EncodePayload + DecodePayload + Sized,
{
    RequestNext,
    AwaitReply,
    RollForward(C, Tip),
    RollBackward(Point, Tip),
    FindIntersect(Vec<Point>),
    IntersectFound(Point, Tip),
    IntersectNotFound(Tip),
    Done,
}
