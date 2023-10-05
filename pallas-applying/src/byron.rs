//! Utilities required for Byron-era transaction validation.

use crate::types::{ByronProtParams, UTxOs, ValidationError, ValidationResult};

use pallas_primitives::byron::{MintedTxPayload, Tx};

// TODO: implement missing validation rules.
pub fn validate_byron_tx(
    mtxp: &MintedTxPayload,
    _utxos: &UTxOs,
    _prot_pps: &ByronProtParams,
) -> ValidationResult {
    check_ins_not_empty(&mtxp.transaction)
}

fn check_ins_not_empty(tx: &Tx) -> ValidationResult {
    if tx.inputs.clone().to_vec().is_empty() {
        return Err(ValidationError::TxInsEmpty);
    }
    Ok(())
}
