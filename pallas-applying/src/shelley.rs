//! Utilities required for Shelley-era transaction validation.

use crate::types::{ShelleyProtParams, UTxOs, ValidationError, ValidationResult};

use pallas_primitives::alonzo::{MintedTx, TransactionBody};

// TODO: implement each of the validation rules.
pub fn validate_shelley_tx(
    mtx: &MintedTx,
    _utxos: &UTxOs,
    _prot_pps: &ShelleyProtParams,
    _prot_magic: &u32,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body;
    check_ins_not_empty(tx_body)
}

fn check_ins_not_empty(tx_body: &TransactionBody) -> ValidationResult {
    if tx_body.inputs.is_empty() {
        return Err(ValidationError::TxInsEmpty);
    }
    Ok(())
}
