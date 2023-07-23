use minicbor::{Decode, Encode};
use pallas_crypto::hash::Hash;
use pallas_primitives::{
    babbage::{AuxiliaryData, TransactionBody, TransactionInput, WitnessSet},
    Fragment,
};

mod output;

pub use output::*;

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

#[derive(Debug, Encode, Decode, Clone)]
pub struct Transaction {
    #[n(0)]
    pub body: TransactionBody,
    #[n(1)]
    pub witness_set: WitnessSet,
    #[n(2)]
    pub is_valid: bool,
    #[n(3)]
    pub auxiliary_data: Option<AuxiliaryData>,
}

impl Transaction {
    pub fn hex_encoded(self) -> Result<String, pallas_primitives::Error> {
        self.encode_fragment().map(hex::encode)
    }
}
