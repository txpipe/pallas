use pallas_crypto::hash::Hash;
use pallas_primitives::babbage::TransactionInput;

#[derive(Debug, Clone)]
pub struct Input {
    transaction_id: Hash<32>,
    index: u64,
}

impl Input {
    pub fn new(transaction_id: impl Into<Hash<32>>, index: u64) -> Self {
        Self {
            transaction_id: transaction_id.into(),
            index,
        }
    }

    pub fn build(self) -> TransactionInput {
        TransactionInput {
            transaction_id: self.transaction_id,
            index: self.index,
        }
    }
}
