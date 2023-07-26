use pallas_traverse::wellknown::GenesisValues;

mod builder;
mod fee;
mod strategy;
mod transaction;

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
        (timestamp / self.genesis_values.shelley_slot_length as u64)
            .checked_sub(self.genesis_values.shelley_known_time)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// The built transaction has no given inputs
    NoInputs,

    /// The built transaction has no outputs
    NoOutputs,

    /// The timestamp provided for either the `.valid_after` or `.valid_until` methods of the
    /// builder are not valid. This usually happens because the provided timestamp comes before the
    /// Shelley hardfork, hence it is not possible to generate a slot number for it.
    InvalidTimestamp,

    /// The transaction can not be encoded to CBOR.
    /// This should not happen usually, only if it is invalid UTF-8. We don't want to panic in those
    /// unusual cases, just return to callee so they can retry.
    UnencodableTransaction,
}
