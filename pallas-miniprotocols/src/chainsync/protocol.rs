use std::{fmt::Debug, ops::Deref};

use crate::common::Point;

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub struct HeaderContent {
    pub variant: u8,
    pub byron_prefix: Option<(u8, u64)>,
    pub cbor: Vec<u8>,
}

#[derive(Debug)]
pub struct BlockContent(pub Vec<u8>);

impl Deref for BlockContent {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Into<Vec<u8>> for BlockContent {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

#[derive(Debug)]
pub struct SkippedContent;
