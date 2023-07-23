use pallas_traverse::fees::{compute_linear_fee_policy, PolicyParams};

use crate::{transaction::Transaction, ValidationError};

pub struct Fee;

impl Fee {
    pub fn linear() -> LinearFee {
        LinearFee
    }
}

pub struct LinearFee;

impl LinearFee {
    pub fn calculate(&self, tx: &Transaction) -> Result<u64, ValidationError> {
        let len = tx
            .hex_encoded()
            .map_err(|_| ValidationError::UnencodableTransaction)?
            .len() as u64;

        Ok(compute_linear_fee_policy(len, &PolicyParams::default()))
    }
}
