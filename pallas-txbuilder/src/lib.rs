use pallas_primitives::babbage::NetworkId;

mod builder;
mod prelude;
mod strategy;
mod transaction;

pub struct NetworkParams {
    network_id: NetworkId,
    min_utxo_value: u64,
}

impl Default for NetworkParams {
    fn default() -> Self {
        Self {
            network_id: NetworkId::One,
            min_utxo_value: 1000000,
        }
    }
}
