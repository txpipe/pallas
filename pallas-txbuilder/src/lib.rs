use pallas_primitives::babbage::NetworkId;

mod builder;
mod strategy;
mod transaction;

pub mod prelude;

pub struct NetworkParams {
    pub network_id: NetworkId,
    pub min_utxo_value: u64,
}

impl Default for NetworkParams {
    fn default() -> Self {
        Self {
            network_id: NetworkId::One,
            min_utxo_value: 1000000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    TransactionUnbalanced,
}
