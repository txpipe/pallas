use std::fmt::Debug;

/// Well-known magic for testnet
pub const TESTNET_MAGIC: u64 = 1097911063;

/// Well-known magic for mainnet
pub const MAINNET_MAGIC: u64 = 764824073;

/// A point within a chain
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Point(pub u64, pub Vec<u8>);

impl Debug for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Point")
            .field(&self.0)
            .field(&hex::encode(&self.1))
            .finish()
    }
}

impl Point {
    pub fn new(slot: u64, hash: Vec<u8>) -> Self {
        Point(slot, hash)
    }
}
