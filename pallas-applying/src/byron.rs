//! Utilities required for Byron-era transaction validation.

use crate::types::{ByronProtParams, MultiEraInput, UTxOs, ValidationError, ValidationResult};

use pallas_primitives::byron::{MintedTxPayload, Tx};

// TODO: implement missing validation rules.
pub fn validate_byron_tx(
    mtxp: &MintedTxPayload,
    utxos: &UTxOs,
    _prot_pps: &ByronProtParams,
) -> ValidationResult {
    let tx: &Tx = &mtxp.transaction;
    check_ins_not_empty(tx)?;
    check_outs_not_empty(tx)?;
    check_ins_in_utxos(tx, utxos)
}

fn check_ins_not_empty(tx: &Tx) -> ValidationResult {
    if tx.inputs.clone().to_vec().is_empty() {
        return Err(ValidationError::TxInsEmpty);
    }
    Ok(())
}

fn check_outs_not_empty(tx: &Tx) -> ValidationResult {
    if tx.outputs.clone().to_vec().is_empty() {
        return Err(ValidationError::TxOutsEmpty);
    }
    Ok(())
}

fn check_ins_in_utxos(tx: &Tx, utxos: &UTxOs) -> ValidationResult {
    for input in tx.inputs.iter() {
        if !(utxos.contains_key(&MultiEraInput::from_byron(input))) {
            return Err(ValidationError::InputMissingInUTxO);
        }
    }
    Ok(())
}
