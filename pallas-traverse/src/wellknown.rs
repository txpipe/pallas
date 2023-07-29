use serde::{Deserialize, Serialize};

/// Well-known magic for testnet
pub const TESTNET_MAGIC: u64 = 1097911063;

/// Well-known magic for mainnet
pub const MAINNET_MAGIC: u64 = 764824073;

/// Well-known magic for preview
pub const PREVIEW_MAGIC: u64 = 2;

/// Well-known magic for pre-production
pub const PRE_PRODUCTION_MAGIC: u64 = 1;

/// Well-known information about specific networks
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenesisValues {
    pub magic: u64,
    pub byron_epoch_length: u32,
    pub byron_slot_length: u32,
    pub byron_known_slot: u64,
    pub byron_known_hash: String,
    pub byron_known_time: u64,
    pub shelley_epoch_length: u32,
    pub shelley_slot_length: u32,
    pub shelley_known_slot: u64,
    pub shelley_known_hash: String,
    pub shelley_known_time: u64,
}

impl GenesisValues {
    /// Hardcoded values for mainnet
    pub fn mainnet() -> Self {
        GenesisValues {
            magic: MAINNET_MAGIC,
            byron_epoch_length: 432000,
            byron_slot_length: 20,
            byron_known_slot: 0,
            byron_known_time: 1506203091,
            byron_known_hash: "f0f7892b5c333cffc4b3c4344de48af4cc63f55e44936196f365a9ef2244134f"
                .to_string(),
            shelley_epoch_length: 432000,
            shelley_slot_length: 1,
            shelley_known_slot: 4492800,
            shelley_known_hash: "aa83acbf5904c0edfe4d79b3689d3d00fcfc553cf360fd2229b98d464c28e9de"
                .to_string(),
            shelley_known_time: 1596059091,
        }
    }

    /// Hardcoded values for testnet
    pub fn testnet() -> Self {
        GenesisValues {
            magic: TESTNET_MAGIC,
            byron_epoch_length: 432000,
            byron_slot_length: 20,
            byron_known_slot: 0,
            byron_known_time: 1564010416,
            byron_known_hash: "8f8602837f7c6f8b8867dd1cbc1842cf51a27eaed2c70ef48325d00f8efb320f"
                .to_string(),
            shelley_epoch_length: 432000,
            shelley_slot_length: 1,
            shelley_known_slot: 1598400,
            shelley_known_hash: "02b1c561715da9e540411123a6135ee319b02f60b9a11a603d3305556c04329f"
                .to_string(),
            shelley_known_time: 1595967616,
        }
    }

    pub fn preview() -> Self {
        GenesisValues {
            magic: PREVIEW_MAGIC,
            byron_epoch_length: 432000,
            byron_slot_length: 20,
            byron_known_slot: 0,
            byron_known_hash: "".to_string(),
            byron_known_time: 1660003200,
            shelley_epoch_length: 432000,
            shelley_slot_length: 1,
            shelley_known_slot: 25260,
            shelley_known_hash: "cac921895ef5f2e85f7e6e6b51b663ab81b3605cd47d6b6d66e8e785e5c65011"
                .to_string(),
            shelley_known_time: 1660003200,
        }
    }

    /// Hardcoded values for the "pre-prod" testnet
    pub fn preprod() -> Self {
        GenesisValues {
            magic: PRE_PRODUCTION_MAGIC,
            byron_epoch_length: 432000,
            byron_slot_length: 20,
            byron_known_slot: 0,
            byron_known_hash: "9ad7ff320c9cf74e0f5ee78d22a85ce42bb0a487d0506bf60cfb5a91ea4497d2"
                .to_string(),
            byron_known_time: 1654041600,
            shelley_epoch_length: 432000,
            shelley_slot_length: 1,
            shelley_known_slot: 86400,
            shelley_known_hash: "c4a1595c5cc7a31eda9e544986fe9387af4e3491afe0ca9a80714f01951bbd5c"
                .to_string(),
            shelley_known_time: 1654041600,
        }
    }

    /// Uses the value of the magic to return either mainnet or testnet
    /// hardcoded values.
    pub fn from_magic(magic: u64) -> Option<GenesisValues> {
        match magic {
            MAINNET_MAGIC => Some(Self::mainnet()),
            TESTNET_MAGIC => Some(Self::testnet()),
            PREVIEW_MAGIC => Some(Self::preview()),
            PRE_PRODUCTION_MAGIC => Some(Self::preprod()),
            _ => None,
        }
    }
}

impl Default for GenesisValues {
    fn default() -> Self {
        Self::mainnet()
    }
}
