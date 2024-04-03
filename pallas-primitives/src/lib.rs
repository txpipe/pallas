//! Ledger primitives and cbor codec for the Cardano eras

mod framework;

pub mod alonzo;
pub mod babbage;
pub mod byron;
pub mod conway;

pub use framework::*;

use pallas_codec::minicbor::{self, Decode, Encode};

#[derive(Debug, Clone, Encode, Decode)]
pub struct FeePolicy {
    #[n(0)]
    pub summand: u64,

    #[n(1)]
    pub multiplier: u64,
}
