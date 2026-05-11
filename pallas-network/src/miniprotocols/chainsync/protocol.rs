use std::{fmt::Debug, ops::Deref};

use crate::miniprotocols::Point;

/// The tip of a chain, characterized by a point and its block height.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tip(pub Point, pub u64);

/// Result of [`Message::FindIntersect`]: the agreed-upon point (if any) plus the peer's current tip.
pub type IntersectResponse = (Option<Point>, Tip);

/// Chain-sync state machine state.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    /// Idle: client may request next or look for an intersection.
    Idle,
    /// Server has been asked for next; peer may already have one to send.
    CanAwait,
    /// Server is awaiting a new block before replying.
    MustReply,
    /// Client is awaiting an intersect response.
    Intersect,
    /// Protocol is terminated.
    Done,
}

/// A generic chain-sync message for either header or block content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message<C> {
    /// Client → server: deliver the next chain update.
    RequestNext,
    /// Server → client: no update available right now; wait for one.
    AwaitReply,
    /// Server → client: chain advances with new content and the current tip.
    RollForward(C, Tip),
    /// Server → client: roll back to the given point, with the current tip.
    RollBackward(Point, Tip),
    /// Client → server: find the most recent point in this list that the server has.
    FindIntersect(Vec<Point>),
    /// Server → client: intersection found at the given point, with the current tip.
    IntersectFound(Point, Tip),
    /// Server → client: none of the offered points are on the server's chain.
    IntersectNotFound(Tip),
    /// Client → server: terminate the protocol.
    Done,
}

/// Block header content delivered by node-to-node chain-sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderContent {
    /// Era tag from the CBOR multi-era wrapper.
    pub variant: u8,
    /// Byron-only payload prefix when `variant` is a Byron variant.
    pub byron_prefix: Option<(u8, u64)>,
    /// Era-specific header CBOR bytes.
    pub cbor: Vec<u8>,
}

/// Whole-block content delivered by node-to-client chain-sync.
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

/// Placeholder content for chain-sync flavors that omit the body payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedContent;
