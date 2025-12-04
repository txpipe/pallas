use std::{fmt::Debug, ops::Deref};

use crate::miniprotocols::Point;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Tip(pub Point, pub u64);

pub type IntersectResponse = (Option<Point>, Tip);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    Idle,
    CanAwait,
    MustReply,
    Intersect,
    Done,
}

/// A generic chain-sync message for either header or block content
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message<C> {
    RequestNext,
    AwaitReply,
    RollForward(C, Tip),
    RollBackward(Point, Tip),
    FindIntersect(Vec<Point>),
    IntersectFound(Point, Tip),
    IntersectNotFound(Tip),
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderContent {
    pub variant: u8,
    pub byron_prefix: Option<(u8, u64)>,
    pub cbor: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockContent(pub Vec<u8>);

impl Deref for BlockContent {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<BlockContent> for Vec<u8> {
    fn from(other: BlockContent) -> Self {
        other.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedContent;
