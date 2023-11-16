//! Utilities required for Shelley-era transaction validation.

use crate::types::{ShelleyProtParams, UTxOs, ValidationError, ValidationResult};
use pallas_primitives::alonzo::{MintedTx, TransactionBody};
use pallas_traverse::MultiEraInput;

// TODO: implement each of the validation rules.
pub fn validate_shelley_tx(
    mtx: &MintedTx,
    utxos: &UTxOs,
    _prot_pps: &ShelleyProtParams,
    _prot_magic: &u32,
    block_slot: &u64,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body;
    check_ins_not_empty(tx_body)?;
    check_ins_in_utxos(tx_body, utxos)?;
    check_ttl(tx_body, block_slot)
}

fn check_ins_not_empty(tx_body: &TransactionBody) -> ValidationResult {
    if tx_body.inputs.is_empty() {
        return Err(ValidationError::TxInsEmpty);
    }
    Ok(())
}

fn check_ins_in_utxos(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    for input in tx_body.inputs.iter() {
        if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(input))) {
            return Err(ValidationError::InputMissingInUTxO);
        }
    }
    Ok(())
}

fn check_ttl(tx_body: &TransactionBody, block_slot: &u64) -> ValidationResult {
    match tx_body.ttl {
        Some(ttl) => {
            if ttl < *block_slot {
                Err(ValidationError::TTLExceeded)
            } else {
                Ok(())
            }
        }
        None => Err(ValidationError::AlonzoCompatibleNotShelley),
    }
}
