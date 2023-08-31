use pallas_traverse::wellknown::GenesisValues;

mod asset;
mod builder;
mod fee;
mod transaction;

pub mod plutus;
pub mod prelude;

#[derive(Debug, Clone)]
pub struct NetworkParams {
    pub genesis_values: GenesisValues,
}

impl NetworkParams {
    pub fn mainnet() -> Self {
        Self {
            genesis_values: GenesisValues::mainnet(),
        }
    }

    pub fn testnet() -> Self {
        Self {
            genesis_values: GenesisValues::testnet(),
        }
    }

    pub fn network_id(&self) -> u64 {
        self.genesis_values.network_id
    }

    pub fn timestamp_to_slot(&self, timestamp: u64) -> Option<u64> {
        timestamp
            .checked_sub(self.genesis_values.shelley_known_time)
            .map(|x| x / self.genesis_values.shelley_slot_length as u64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    /// The built transaction has no given inputs
    #[error("Transaction has no inputs")]
    NoInputs,

    /// The built transaction has no outputs
    #[error("Transaction has no outputs")]
    NoOutputs,

    /// The timestamp provided for either the `.valid_after` or `.valid_until` methods of the
    /// builder are not valid. This usually happens because the provided timestamp comes before the
    /// Shelley hardfork, hence it is not possible to generate a slot number for it.
    #[error("Invalid timestamp")]
    InvalidTimestamp,

    /// The transaction can not be encoded to CBOR.
    /// This should not happen usually, only if it is invalid UTF-8. We don't want to panic in those
    /// unusual cases, just return to callee so they can retry.
    #[error("Unencodable transaction")]
    UnencodableTransaction,

    #[error("Asset error {0}")]
    AssetError(#[from] asset::AssetError),

    /// The transaction at least one invalid collateral input
    ///
    /// Transactions can only have pure-ada UTXOs as collateral returns, this happens if any are
    /// multi-asset.
    #[error("Invalid collateral input")]
    InvalidCollateralInput,

    /// The transaction has an invalid collateral return UTXO
    ///
    /// Transactions can only have pure-ada UTXOs as collaterals, this happens if any are
    /// multi-asset.
    #[error("Invalid collateral return output")]
    InvalidCollateralReturn,
}
