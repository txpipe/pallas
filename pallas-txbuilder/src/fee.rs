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
    pub fn with_fee(&self, mut tx: Transaction) -> Result<Transaction, ValidationError> {
        let mut len;
        let mut calculated_fee;

        // We calculate a fee to the transaction, then recalculate the fee using the new amount,
        // until it stops changing. This is necessary because the fee is part of the transaction
        // bytes, and re-doing it multiple times it the most accurate we can get.
        loop {
            len = tx.hex_encoded()?.len() as u64;
            calculated_fee = compute_linear_fee_policy(len, &PolicyParams::default());

            if tx.body.fee == calculated_fee {
                break;
            }

            tx.body.fee = calculated_fee;
        }

        tx.body.fee = calculated_fee;

        Ok(tx)
    }
}
