//! Utilities required for Shelley-era transaction validation.

use crate::types::{
    AlonzoError::*,
    AlonzoProtParams, UTxOs,
    ValidationError::{self, *},
    ValidationResult,
};
use pallas_codec::{minicbor::encode, utils::KeepRaw};
use pallas_primitives::alonzo::{
    MintedTx, MintedWitnessSet, NativeScript, PlutusData, PlutusScript, TransactionBody,
    VKeyWitness,
};

pub fn validate_alonzo_tx(
    mtx: &MintedTx,
    utxos: &UTxOs,
    prot_pps: &AlonzoProtParams,
    network_id: &u8,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body;
    let size: &u64 = &get_tx_size(tx_body)?;
    check_ins_not_empty(tx_body)?;
    check_ins_and_collateral_in_utxos(tx_body, utxos)?;
    check_tx_validity_interval(tx_body, mtx)?;
    check_fees(tx_body, size, mtx, utxos, prot_pps)?;
    check_preservation_of_value(tx_body, utxos, prot_pps)?;
    check_min_lovelace(tx_body, prot_pps)?;
    check_output_values_size(tx_body, prot_pps)?;
    check_network_id(tx_body, network_id)?;
    check_tx_size(size, prot_pps)?;
    check_tx_ex_units(mtx, prot_pps)?;
    check_witnesses(tx_body, utxos, mtx)?;
    check_languages(mtx, prot_pps)?;
    check_metadata(tx_body, mtx)?;
    check_script_data_hash(tx_body, mtx, prot_pps)?;
    check_minting(tx_body, mtx)
}

fn get_tx_size(tx_body: &TransactionBody) -> Result<u64, ValidationError> {
    let mut buff: Vec<u8> = Vec::new();
    match encode(tx_body, &mut buff) {
        Ok(()) => Ok(buff.len() as u64),
        Err(_) => Err(Alonzo(UnknownTxSize)),
    }
}

// The set of transaction inputs is not empty.
fn check_ins_not_empty(_tx_body: &TransactionBody) -> ValidationResult {
    Ok(())
}

// All transaction inputs and collateral inputs are in the set of (yet) unspent transaction outputs.
fn check_ins_and_collateral_in_utxos(
    _tx_body: &TransactionBody,
    _utxos: &UTxOs,
) -> ValidationResult {
    Ok(())
}

// The block slot is contained in the transaction validity interval.
fn check_tx_validity_interval(tx_body: &TransactionBody, mtx: &MintedTx) -> ValidationResult {
    check_upper_bound_necessity(tx_body, mtx)?;
    Ok(())
}

// The upper bound of the validity time interval is suitable for script execution.
fn check_upper_bound_necessity(_tx_body: &TransactionBody, _mtx: &MintedTx) -> ValidationResult {
    Ok(())
}

fn check_fees(
    tx_body: &TransactionBody,
    size: &u64,
    mtx: &MintedTx,
    utxos: &UTxOs,
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    check_min_fees(tx_body, size, prot_pps)?;
    if presence_of_plutus_scripts(mtx) {
        check_collaterals(tx_body, utxos, prot_pps)?
    }
    Ok(())
}

// The fee paid by the transaction should be greater than or equal to the minimum fee.
fn check_min_fees(
    _tx_body: &TransactionBody,
    _size: &u64,
    _prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    Ok(())
}

fn presence_of_plutus_scripts(_mtx: &MintedTx) -> bool {
    true
}

fn check_collaterals(
    tx_body: &TransactionBody,
    _utxos: &UTxOs,
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    check_collaterals_number(tx_body, prot_pps)?;
    check_collaterals_address(tx_body)?;
    check_collaterals_only_contain_lovelace(tx_body)?;
    check_collaterals_percentage(tx_body, prot_pps)
}

// The set of collateral inputs is not empty.
// The number of collateral inputs is below maximum allowed by protocol.
fn check_collaterals_number(
    _tx_body: &TransactionBody,
    _prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    Ok(())
}

// Each collateral input refers to a verification-key address.
fn check_collaterals_address(_tx_body: &TransactionBody) -> ValidationResult {
    Ok(())
}

// Collateral inputs contain only ADA.
fn check_collaterals_only_contain_lovelace(_tx_body: &TransactionBody) -> ValidationResult {
    Ok(())
}

// The total lovelace contained in collateral inputs should be greater than or equal to the minimum
// fee percentage.
fn check_collaterals_percentage(
    _tx_body: &TransactionBody,
    _prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    Ok(())
}

// The preservation of value property holds.
fn check_preservation_of_value(
    _tx_body: &TransactionBody,
    _utxos: &UTxOs,
    _prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    Ok(())
}

// All transaction outputs should contain at least the minimum lovelace.
fn check_min_lovelace(
    _tx_body: &TransactionBody,
    _prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    Ok(())
}

// The size of the value in each of the outputs should not be greater than the maximum allowed.
fn check_output_values_size(
    _tx_body: &TransactionBody,
    _prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    Ok(())
}

// The network ID of the transaction and its output addresses is correct.
fn check_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    check_tx_outs_network_id(tx_body, network_id)?;
    check_tx_network_id(tx_body, network_id)
}

// The network ID of each output matches the global network ID.
fn check_tx_outs_network_id(_tx_body: &TransactionBody, _network_id: &u8) -> ValidationResult {
    Ok(())
}

// The network ID of the transaction body is either undefined or equal to the global network ID.
fn check_tx_network_id(_tx_body: &TransactionBody, _network_id: &u8) -> ValidationResult {
    Ok(())
}

// The transaction size does not exceed the protocol limit.
fn check_tx_size(_size: &u64, _prot_pps: &AlonzoProtParams) -> ValidationResult {
    Ok(())
}

// The number of execution units of the transaction should not exceed the maximum allowed.
fn check_tx_ex_units(_mtx: &MintedTx, _prot_pps: &AlonzoProtParams) -> ValidationResult {
    Ok(())
}

fn check_witnesses(tx_body: &TransactionBody, utxos: &UTxOs, mtx: &MintedTx) -> ValidationResult {
    let tx_wits: &MintedWitnessSet = &mtx.transaction_witness_set;
    check_needed_scripts_are_included(
        tx_body,
        utxos,
        &tx_wits.native_script,
        &tx_wits.plutus_script,
    )?;
    check_datums(tx_body, utxos, &tx_wits.plutus_data)?;
    check_redeemers(tx_body, utxos, tx_wits)?;
    check_witnesses_for_verification_key_inputs(tx_body, utxos, &tx_wits.vkeywitness)?;
    check_required_signers(tx_body, utxos, &tx_wits.vkeywitness)
}

// The set of needed scripts (minting policies, native scripts and Plutus scripts needed to validate
// the transaction) equals the set of scripts contained in the transaction witnesses set.
fn check_needed_scripts_are_included(
    _tx_body: &TransactionBody,
    _utxos: &UTxOs,
    _native_script_wits: &Option<Vec<NativeScript>>,
    _plutus_script_wits: &Option<Vec<PlutusScript>>,
) -> ValidationResult {
    Ok(())
}

fn check_datums(
    tx_body: &TransactionBody,
    _utxos: &UTxOs,
    plutus_data: &Option<Vec<KeepRaw<PlutusData>>>,
) -> ValidationResult {
    check_input_datum_hash_in_witness_set(tx_body, plutus_data)?;
    check_datums_from_witness_set_in_inputs_or_output(tx_body, plutus_data)
}

// Each datum hash in a Plutus script input matches the hash of a datum in the transaction witness
// set.
fn check_input_datum_hash_in_witness_set(
    _tx_body: &TransactionBody,
    _plutus_data: &Option<Vec<KeepRaw<PlutusData>>>,
) -> ValidationResult {
    Ok(())
}

// Each datum object in the transaction witness set corresponds either to an output datum hash or to
// the datum hash of a Plutus script input.
fn check_datums_from_witness_set_in_inputs_or_output(
    _tx_body: &TransactionBody,
    _plutus_data: &Option<Vec<KeepRaw<PlutusData>>>,
) -> ValidationResult {
    Ok(())
}

// The set of redeemers in the transaction witness set should match the set of Plutus scripts needed
// to validate the transaction.
fn check_redeemers(
    _tx_body: &TransactionBody,
    _utxos: &UTxOs,
    _tx_wits: &MintedWitnessSet,
) -> ValidationResult {
    Ok(())
}

// The owner of each transaction input and each collateral input should have signed the transaction.
fn check_witnesses_for_verification_key_inputs(
    _tx_body: &TransactionBody,
    _utxos: &UTxOs,
    _vkey_wits: &Option<Vec<VKeyWitness>>,
) -> ValidationResult {
    Ok(())
}

// All required signers (needed by a Plutus script) have a corresponding match in the transaction
// witness set.
fn check_required_signers(
    _tx_body: &TransactionBody,
    _utxos: &UTxOs,
    _vkey_wits: &Option<Vec<VKeyWitness>>,
) -> ValidationResult {
    Ok(())
}

// The required script languages are included in the protocol parameters.
fn check_languages(_mtx: &MintedTx, _prot_pps: &AlonzoProtParams) -> ValidationResult {
    Ok(())
}

// The metadata of the transaction is valid.
fn check_metadata(_tx_body: &TransactionBody, _mtx: &MintedTx) -> ValidationResult {
    Ok(())
}

// The script data integrity hash matches the hash of the redeemers, languages and datums of the
// transaction witness set.
fn check_script_data_hash(
    _tx_body: &TransactionBody,
    _mtx: &MintedTx,
    _prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    Ok(())
}

// Each minted / burned asset is paired with an appropriate native script or minting policy.
fn check_minting(tx_body: &TransactionBody, _mtx: &MintedTx) -> ValidationResult {
    check_ada_not_minted(tx_body)?;
    Ok(())
}

// No ADA is minted.
fn check_ada_not_minted(_tx_body: &TransactionBody) -> ValidationResult {
    Ok(())
}
