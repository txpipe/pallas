use minicbor::{Decode, Encode};
use pallas_codec::utils::Bytes;
use pallas_crypto::hash::Hash;
use pallas_primitives::{
    babbage::{
        AuxiliaryData, PseudoPostAlonzoTransactionOutput, TransactionBody, TransactionInput,
        TransactionOutput, Value, WitnessSet,
    },
    Fragment,
};

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

#[derive(Debug, Clone)]
pub enum Output {
    Lovelaces { address: Bytes, value: u64 },
}

impl Output {
    pub fn lovelaces(address: impl Into<Bytes>, value: u64) -> Self {
        Self::Lovelaces {
            address: address.into(),
            value,
        }
    }

    pub fn build(self) -> TransactionOutput {
        match self {
            Self::Lovelaces { address, value } => {
                TransactionOutput::PostAlonzo(PseudoPostAlonzoTransactionOutput {
                    address,
                    value: Value::Coin(value),
                    datum_option: None,
                    script_ref: None,
                })
            }
        }
    }
}

#[derive(Encode, Decode, Clone)]
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
