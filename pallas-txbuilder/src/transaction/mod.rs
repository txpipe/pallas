use minicbor::{Decode, Encode};
use pallas_primitives::{
    babbage::{AuxiliaryData, TransactionBody, WitnessSet},
    Fragment,
};

mod input;
mod output;

pub use input::*;
pub use output::*;

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
    pub fn hex_encoded(&self) -> Result<String, pallas_primitives::Error> {
        self.encode_fragment().map(hex::encode)
    }
}
