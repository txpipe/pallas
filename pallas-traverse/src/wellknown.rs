use serde::{Deserialize, Serialize};

/// Well-known params for testnet
pub const TESTNET_MAGIC: u64 = 1097911063;
pub const TESTNET_NETWORK_ID: u64 = 0;

/// Well-known params for mainnet
pub const MAINNET_MAGIC: u64 = 764824073;
pub const MAINNET_NETWORK_ID: u64 = 1;

/// Well-known params for preview
pub const PREVIEW_MAGIC: u64 = 2;
pub const PREVIEW_NETWORK_ID: u64 = 0;

/// Well-known params for pre-production
pub const PRE_PRODUCTION_MAGIC: u64 = 1;
pub const PRE_PRODUCTION_NETWORK_ID: u64 = 0;

/// Well-known information about specific networks
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenesisValues {
    pub magic: u64,
    pub network_id: u64,
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
            network_id: MAINNET_NETWORK_ID,
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
            network_id: TESTNET_NETWORK_ID,
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
            network_id: PREVIEW_NETWORK_ID,
            byron_epoch_length: 86400,
            byron_slot_length: 20,
            byron_known_slot: 0,
            byron_known_hash: "".to_string(),
            byron_known_time: 1666656000,
            shelley_epoch_length: 86400,
            shelley_slot_length: 1,
            shelley_known_slot: 0,
            shelley_known_hash: "268ae601af8f9214804735910a3301881fbe0eec9936db7d1fb9fc39e93d1e37"
                .to_string(),
            shelley_known_time: 1666656000,
        }
    }

    /// Hardcoded values for the "pre-prod" testnet
    pub fn preprod() -> Self {
        GenesisValues {
            magic: PRE_PRODUCTION_MAGIC,
            network_id: PRE_PRODUCTION_NETWORK_ID,
            byron_epoch_length: 432000,
            byron_slot_length: 20,
            byron_known_slot: 0,
            byron_known_hash: "9ad7ff320c9cf74e0f5ee78d22a85ce42bb0a487d0506bf60cfb5a91ea4497d2"
                .to_string(),
            byron_known_time: 1654041600,
            shelley_epoch_length: 432000,
            shelley_slot_length: 1,
            shelley_known_slot: 86400,
            shelley_known_hash: "c971bfb21d2732457f9febf79d9b02b20b9a3bef12c561a78b818bcb8b35a574"
                .to_string(),
            shelley_known_time: 1655769600,
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
