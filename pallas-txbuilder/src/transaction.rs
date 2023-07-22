use minicbor::{Decode, Encode};
use pallas_primitives::babbage::{AuxiliaryData, PolicyId, TransactionBody, WitnessSet};

pub enum Input {
    Lovelaces(u64),
    Asset(PolicyId, u64),
}

impl Input {
    pub fn lovelaces(amount: u64) -> Self {
        Self::Lovelaces(amount)
    }

    pub fn asset(policy_id: &str, amount: u64) -> Self {
        todo!()
    }
}

pub struct Output {}

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
