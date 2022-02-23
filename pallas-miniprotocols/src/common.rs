use std::fmt::Debug;

/// Well-known magic for testnet
pub const TESTNET_MAGIC: u64 = 1097911063;

/// Well-known magic for mainnet
pub const MAINNET_MAGIC: u64 = 764824073;

/// A point within a chain
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum Point {
    Origin,
    Specific(u64, Vec<u8>),
}

impl Debug for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Origin => write!(f, "Origin"),
            Self::Specific(arg0, arg1) => write!(f, "({}, {})", arg0, hex::encode(arg1)),
        }
    }
}

impl Point {
    pub fn new(slot: u64, hash: Vec<u8>) -> Self {
        Point::Specific(slot, hash)
    }
}
