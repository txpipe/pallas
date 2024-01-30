//! Utilities required for Babbage-era transaction validation.

use crate::utils::{
    get_babbage_tx_size, BabbageError::*, BabbageProtParams, UTxOs, ValidationError::*,
    ValidationResult,
};
use pallas_primitives::babbage::{MintedTransactionBody, MintedTx};
use pallas_traverse::MultiEraInput;

pub fn validate_babbage_tx(
    mtx: &MintedTx,
    utxos: &UTxOs,
    prot_pps: &BabbageProtParams,
    block_slot: &u64,
    network_id: &u8,
) -> ValidationResult {
    let tx_body: &MintedTransactionBody = &mtx.transaction_body.clone();
    let size: &u64 = &get_babbage_tx_size(tx_body).ok_or(Babbage(UnknownTxSize))?;
    check_ins_not_empty(tx_body)?;
    check_all_ins_in_utxos(tx_body, utxos)?;
    check_tx_validity_interval(tx_body, block_slot)?;
    check_fee(tx_body, size, mtx, utxos, prot_pps)?;
    check_preservation_of_value(tx_body, utxos)?;
    check_min_lovelace(tx_body, prot_pps)?;
    check_output_val_size(tx_body, prot_pps)?;
    check_network_id(tx_body, network_id)?;
    check_tx_size(size, prot_pps)?;
    check_tx_ex_units(mtx, prot_pps)?;
    check_minting(tx_body, mtx)?;
    check_well_formedness(tx_body, mtx)?;
    check_witness_set(mtx, utxos)?;
    check_languages(mtx, prot_pps)?;
    check_auxiliary_data(tx_body, mtx)?;
    check_script_data_hash(tx_body, mtx)
}

// The set of transaction inputs is not empty.
fn check_ins_not_empty(tx_body: &MintedTransactionBody) -> ValidationResult {
    if tx_body.inputs.is_empty() {
        return Err(Babbage(TxInsEmpty));
    }
    Ok(())
}

// All transaction inputs, collateral inputs and reference inputs are in the
// UTxO set.
fn check_all_ins_in_utxos(tx_body: &MintedTransactionBody, utxos: &UTxOs) -> ValidationResult {
    for input in tx_body.inputs.iter() {
        if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(input))) {
            return Err(Babbage(InputNotInUTxO));
        }
    }
    match &tx_body.collateral {
        None => (),
        Some(collaterals) => {
            for collateral in collaterals {
                if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(collateral))) {
                    return Err(Babbage(CollateralNotInUTxO));
                }
            }
        }
    }
    match &tx_body.reference_inputs {
        None => (),
        Some(reference_inputs) => {
            for reference_input in reference_inputs {
                if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(reference_input))) {
                    return Err(Babbage(CollateralNotInUTxO));
                }
            }
        }
    }
    Ok(())
}

// The block slot is contained in the transaction validity interval, and the
// upper bound is translatable to UTC time.
fn check_tx_validity_interval(
    tx_body: &MintedTransactionBody,
    block_slot: &u64,
) -> ValidationResult {
    check_lower_bound(tx_body, block_slot)?;
    check_upper_bound(tx_body, block_slot)
}

// If defined, the lower bound of the validity time interval does not exceed the
// block slot.
fn check_lower_bound(tx_body: &MintedTransactionBody, block_slot: &u64) -> ValidationResult {
    match tx_body.validity_interval_start {
        Some(lower_bound) => {
            if *block_slot < lower_bound {
                Err(Babbage(BlockPrecedesValInt))
            } else {
                Ok(())
            }
        }
        None => Ok(()),
    }
}

// If defined, the upper bound of the validity time interval is not exceeded by
// the block slot, and it is translatable to UTC time.
fn check_upper_bound(tx_body: &MintedTransactionBody, block_slot: &u64) -> ValidationResult {
    match tx_body.ttl {
        Some(upper_bound) => {
            if upper_bound < *block_slot {
                Err(Babbage(BlockExceedsValInt))
            } else {
                // TODO: check that `upper_bound` is translatable to UTC time.
                Ok(())
            }
        }
        None => Ok(()),
    }
}

fn check_fee(
    _tx_body: &MintedTransactionBody,
    _size: &u64,
    _mtx: &MintedTx,
    _utxos: &UTxOs,
    _prot_pps: &BabbageProtParams,
) -> ValidationResult {
    Ok(())
}

fn check_preservation_of_value(
    _tx_body: &MintedTransactionBody,
    _utxos: &UTxOs,
) -> ValidationResult {
    Ok(())
}

fn check_min_lovelace(
    _tx_body: &MintedTransactionBody,
    _prot_pps: &BabbageProtParams,
) -> ValidationResult {
    Ok(())
}

fn check_output_val_size(
    _tx_body: &MintedTransactionBody,
    _prot_pps: &BabbageProtParams,
) -> ValidationResult {
    Ok(())
}

fn check_network_id(tx_body: &MintedTransactionBody, network_id: &u8) -> ValidationResult {
    check_tx_outs_network_id(tx_body, network_id)?;
    check_tx_network_id(tx_body, network_id)
}

fn check_tx_outs_network_id(
    _tx_body: &MintedTransactionBody,
    _network_id: &u8,
) -> ValidationResult {
    Ok(())
}

fn check_tx_network_id(_tx_body: &MintedTransactionBody, _network_id: &u8) -> ValidationResult {
    Ok(())
}

fn check_tx_size(_size: &u64, _prot_pps: &BabbageProtParams) -> ValidationResult {
    Ok(())
}

fn check_tx_ex_units(_mtx: &MintedTx, _prot_pps: &BabbageProtParams) -> ValidationResult {
    Ok(())
}

fn check_minting(_tx_body: &MintedTransactionBody, _mtx: &MintedTx) -> ValidationResult {
    Ok(())
}

fn check_well_formedness(_tx_body: &MintedTransactionBody, _mtx: &MintedTx) -> ValidationResult {
    Ok(())
}

fn check_witness_set(_mtx: &MintedTx, _utxos: &UTxOs) -> ValidationResult {
    Ok(())
}

fn check_languages(_mtx: &MintedTx, _prot_pps: &BabbageProtParams) -> ValidationResult {
    Ok(())
}

fn check_auxiliary_data(_tx_body: &MintedTransactionBody, _mtx: &MintedTx) -> ValidationResult {
    Ok(())
}

fn check_script_data_hash(_tx_body: &MintedTransactionBody, _mtx: &MintedTx) -> ValidationResult {
    Ok(())
}
