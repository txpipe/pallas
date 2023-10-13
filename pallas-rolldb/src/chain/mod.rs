use pallas_crypto::hash::Hash;

mod store;

#[cfg(test)]
mod tests;

pub type BlockSlot = u64;
pub type BlockHash = Hash<32>;
pub type BlockBody = Vec<u8>;

pub use store::*;
