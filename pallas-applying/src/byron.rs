//! Utilities required for Byron-era transaction validation.

use crate::types::{
    ByronProtParams, MultiEraInput, MultiEraOutput, UTxOs, ValidationError, ValidationResult,
};

use pallas_codec::minicbor::encode;

use pallas_primitives::byron::{MintedTxPayload, Tx};

// TODO: implement missing validation rules.
pub fn validate_byron_tx(
    mtxp: &MintedTxPayload,
    utxos: &UTxOs,
    prot_pps: &ByronProtParams,
) -> ValidationResult {
    let tx: &Tx = &mtxp.transaction;
    let size: u64 = get_tx_size(tx)?;
    check_ins_not_empty(tx)?;
    check_outs_not_empty(tx)?;
    check_ins_in_utxos(tx, utxos)?;
    check_outs_have_lovelace(tx)?;
    check_fees(tx, &size, utxos, prot_pps)?;
    check_size(&size, prot_pps)
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

fn check_outs_have_lovelace(tx: &Tx) -> ValidationResult {
    for output in tx.outputs.iter() {
        if output.amount == 0 {
            return Err(ValidationError::OutputWithoutLovelace);
        }
    }
    Ok(())
}

fn check_fees(tx: &Tx, size: &u64, utxos: &UTxOs, prot_pps: &ByronProtParams) -> ValidationResult {
    let mut inputs_balance: u64 = 0;
    for input in tx.inputs.iter() {
        match utxos
            .get(&MultiEraInput::from_byron(input))
            .and_then(MultiEraOutput::as_byron)
        {
            Some(byron_utxo) => inputs_balance += byron_utxo.amount,
            None => return Err(ValidationError::UnableToComputeFees),
        }
    }
    let mut outputs_balance: u64 = 0;
    for output in tx.outputs.iter() {
        outputs_balance += output.amount
    }
    let total_balance: u64 = inputs_balance - outputs_balance;
    let min_fees: u64 = prot_pps.min_fees_const + prot_pps.min_fees_factor * size;
    if total_balance < min_fees {
        return Err(ValidationError::FeesBelowMin);
    }
    Ok(())
}

fn check_size(size: &u64, prot_pps: &ByronProtParams) -> ValidationResult {
    if *size > prot_pps.max_tx_size {
        return Err(ValidationError::MaxTxSizeExceeded);
    }
    Ok(())
}

fn get_tx_size(tx: &Tx) -> Result<u64, ValidationError> {
    let mut buff: Vec<u8> = Vec::new();
    match encode(tx, &mut buff) {
        Ok(()) => Ok(buff.len() as u64),
        Err(_) => Err(ValidationError::UnknownTxSize),
    }
}
