use pallas_crypto::hash::Hash;
use pallas_network::miniprotocols::Point;

pub type BlockSlot = u64;
pub type BlockHash = Hash<32>;
pub type RawBlock = Vec<u8>;

#[derive(Clone)]
pub enum Intersection {
    Tip,
    Origin,
    Breadcrumbs(Vec<Point>),
}

pub trait Cursor: Send + Sync {
    fn intersection(&self) -> Intersection;
}

#[derive(Debug, Clone)]
pub enum UpstreamEvent {
    RollForward(BlockSlot, BlockHash, RawBlock),
    Rollback(Point),
}

// final output port
pub type DownstreamPort<A> = gasket::messaging::OutputPort<A, UpstreamEvent>;
