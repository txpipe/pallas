use minicbor::{Decode, Encode};
use pallas_primitives::babbage::{AuxiliaryData, TransactionBody, WitnessSet};

pub struct Input;
pub struct Output;

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
