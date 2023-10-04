use pallas_crypto::hash::Hash;
use pallas_primitives::babbage::TransactionInput;

#[derive(Debug, Clone)]
pub struct Input;

impl Input {
    pub fn build(transaction_id: impl Into<Hash<32>>, index: u64) -> TransactionInput {
        TransactionInput {
            transaction_id: transaction_id.into(),
            index,
        }
    }
}
