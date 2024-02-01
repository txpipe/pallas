//! Utilities required for Babbage-era transaction validation.

use crate::utils::{
    add_minted_value, add_values, empty_value, get_babbage_tx_size, get_lovelace_from_alonzo_val,
    get_network_id_value, get_payment_part, get_shelley_address, get_val_size_in_words,
    lovelace_diff_or_fail, values_are_equal,
    BabbageError::*,
    BabbageProtParams, FeePolicy, UTxOs,
    ValidationError::{self, *},
    ValidationResult,
};
use pallas_addresses::{ShelleyAddress, ShelleyPaymentPart};
use pallas_codec::utils::Bytes;
use pallas_primitives::babbage::{
    MintedTransactionBody, MintedTx, MintedWitnessSet, PlutusV1Script, PlutusV2Script,
    PseudoTransactionOutput, TransactionInput, Value,
};
use pallas_traverse::{MultiEraInput, MultiEraOutput};
use std::ops::Deref;

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
    tx_body: &MintedTransactionBody,
    size: &u64,
    mtx: &MintedTx,
    utxos: &UTxOs,
    prot_pps: &BabbageProtParams,
) -> ValidationResult {
    check_min_fee(tx_body, size, prot_pps)?;
    if presence_of_plutus_scripts(mtx) {
        check_collaterals(tx_body, mtx, utxos, prot_pps)?
    }
    Ok(())
}

// The fee paid by the transaction should be greater than or equal to the
// minimum fee.
fn check_min_fee(
    tx_body: &MintedTransactionBody,
    size: &u64,
    prot_pps: &BabbageProtParams,
) -> ValidationResult {
    let fee_policy: &FeePolicy = &prot_pps.fee_policy;
    if tx_body.fee < fee_policy.summand + fee_policy.multiplier * size {
        return Err(Babbage(FeeBelowMin));
    }
    Ok(())
}

fn presence_of_plutus_scripts(mtx: &MintedTx) -> bool {
    let minted_witness_set: &MintedWitnessSet = &mtx.transaction_witness_set;
    let plutus_v1_scripts: &[PlutusV1Script] = &minted_witness_set
        .plutus_v1_script
        .clone()
        .unwrap_or_default();
    let plutus_v2_scripts: &[PlutusV2Script] = &minted_witness_set
        .plutus_v2_script
        .clone()
        .unwrap_or_default();
    !plutus_v1_scripts.is_empty() || !plutus_v2_scripts.is_empty()
}

fn check_collaterals(
    tx_body: &MintedTransactionBody,
    mtx: &MintedTx,
    utxos: &UTxOs,
    prot_pps: &BabbageProtParams,
) -> ValidationResult {
    let collaterals: &[TransactionInput] = &tx_body
        .collateral
        .clone()
        .ok_or(Babbage(CollateralMissing))?;
    check_collaterals_number(collaterals, prot_pps)?;
    check_collaterals_address(collaterals, utxos)?;
    check_collaterals_assets(tx_body, mtx, utxos, prot_pps)
}

// The set of collateral inputs is not empty.
// The number of collateral inputs is below maximum allowed by protocol.
fn check_collaterals_number(
    collaterals: &[TransactionInput],
    prot_pps: &BabbageProtParams,
) -> ValidationResult {
    if collaterals.is_empty() {
        Err(Babbage(CollateralMissing))
    } else if collaterals.len() > prot_pps.max_collateral_inputs as usize {
        Err(Babbage(TooManyCollaterals))
    } else {
        Ok(())
    }
}

// Each collateral input refers to a verification-key address.
fn check_collaterals_address(collaterals: &[TransactionInput], utxos: &UTxOs) -> ValidationResult {
    for collateral in collaterals {
        match utxos.get(&MultiEraInput::from_alonzo_compatible(collateral)) {
            Some(multi_era_output) => {
                if let Some(babbage_output) = MultiEraOutput::as_babbage(multi_era_output) {
                    let address: &Bytes = match babbage_output {
                        PseudoTransactionOutput::Legacy(inner) => &inner.address,
                        PseudoTransactionOutput::PostAlonzo(inner) => &inner.address,
                    };
                    if let ShelleyPaymentPart::Script(_) =
                        get_payment_part(address).ok_or(Babbage(InputDecoding))?
                    {
                        return Err(Babbage(CollateralNotVKeyLocked));
                    }
                }
            }
            None => return Err(Babbage(CollateralNotInUTxO)),
        }
    }
    Ok(())
}

// The balance between collateral inputs and output contains only lovelace.
// The balance is not lower than the minimum allowed.
// The balance matches exactly the collateral annotated in the transaction body.
fn check_collaterals_assets(
    tx_body: &MintedTransactionBody,
    mtx: &MintedTx,
    utxos: &UTxOs,
    prot_pps: &BabbageProtParams,
) -> ValidationResult {
    match &tx_body.collateral {
        Some(collaterals) => {
            let mut coll_input: Value = empty_value();
            for collateral in collaterals {
                match utxos.get(&MultiEraInput::from_alonzo_compatible(collateral)) {
                    Some(multi_era_output) => {
                        coll_input = add_values(
                            &coll_input,
                            &val_from_multi_era_output(multi_era_output),
                            &Babbage(NegativeValue),
                        )?
                    }
                    None => return Err(Babbage(CollateralNotInUTxO)),
                }
            }
            let coll_return: Value = match &tx_body.collateral_return {
                Some(PseudoTransactionOutput::Legacy(output)) => output.amount.clone(),
                Some(PseudoTransactionOutput::PostAlonzo(output)) => output.value.clone(),
                None => Value::Coin(0),
            };
            // The balance between collateral inputs and output contains only lovelace.
            let paid_collateral: u64 =
                lovelace_diff_or_fail(&coll_input, &coll_return, &Babbage(NonLovelaceCollateral))?;
            let fee_percentage: u64 = tx_body.fee * prot_pps.collateral_percent;
            // The balance is not lower than the minimum allowed.
            if paid_collateral * 100 < fee_percentage {
                return Err(Babbage(CollateralMinLovelace));
            }
            // The balance matches exactly the collateral annotated in the transaction body.
            if let Some(annotated_collateral) = &mtx.transaction_body.total_collateral {
                if paid_collateral != *annotated_collateral {
                    return Err(Babbage(CollateralAnnotation));
                }
            }
        }
        None => return Err(Babbage(CollateralMissing)),
    }
    Ok(())
}

fn val_from_multi_era_output(multi_era_output: &MultiEraOutput) -> Value {
    match multi_era_output {
        MultiEraOutput::Byron(output) => Value::Coin(output.amount),
        MultiEraOutput::AlonzoCompatible(output) => output.amount.clone(),
        babbage_output => match babbage_output.as_babbage() {
            Some(PseudoTransactionOutput::Legacy(output)) => output.amount.clone(),
            Some(PseudoTransactionOutput::PostAlonzo(output)) => output.value.clone(),
            None => unimplemented!(), /* If this is the case, then it must be that non-exhaustive
                                       * type MultiEraOutput was extended with another variant */
        },
    }
}

// The preservation of value property holds.
fn check_preservation_of_value(tx_body: &MintedTransactionBody, utxos: &UTxOs) -> ValidationResult {
    let input: Value = get_consumed(tx_body, utxos)?;
    let produced: Value = get_produced(tx_body)?;
    let output: Value = add_values(
        &produced,
        &Value::Coin(tx_body.fee),
        &Babbage(NegativeValue),
    )?;
    if let Some(m) = &tx_body.mint {
        add_minted_value(&output, m, &Babbage(NegativeValue))?;
    }
    if !values_are_equal(&input, &output) {
        return Err(Babbage(PreservationOfValue));
    }
    Ok(())
}

fn get_consumed(tx_body: &MintedTransactionBody, utxos: &UTxOs) -> Result<Value, ValidationError> {
    let mut res: Value = empty_value();
    for input in tx_body.inputs.iter() {
        let multi_era_output: &MultiEraOutput = utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .ok_or(Babbage(InputNotInUTxO))?;
        let val: Value = val_from_multi_era_output(multi_era_output);
        res = add_values(&res, &val, &Babbage(NegativeValue))?;
    }
    Ok(res)
}

fn get_produced(tx_body: &MintedTransactionBody) -> Result<Value, ValidationError> {
    let mut res: Value = empty_value();
    for output in tx_body.outputs.iter() {
        match output {
            PseudoTransactionOutput::Legacy(output) => {
                res = add_values(&res, &output.amount, &Babbage(NegativeValue))?
            }
            PseudoTransactionOutput::PostAlonzo(output) => {
                res = add_values(&res, &output.value, &Babbage(NegativeValue))?
            }
        }
    }
    Ok(res)
}

fn check_min_lovelace(
    tx_body: &MintedTransactionBody,
    prot_pps: &BabbageProtParams,
) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let val: &Value = match output {
            PseudoTransactionOutput::Legacy(output) => &output.amount,
            PseudoTransactionOutput::PostAlonzo(output) => &output.value,
        };
        if get_lovelace_from_alonzo_val(val) < compute_min_lovelace(val, prot_pps) {
            return Err(Babbage(MinLovelaceUnreached));
        }
    }
    Ok(())
}

fn compute_min_lovelace(val: &Value, prot_pps: &BabbageProtParams) -> u64 {
    prot_pps.coins_per_utxo_word * (get_val_size_in_words(val) + 160)
}

// The size of the value in each of the outputs should not be greater than the
// maximum allowed.
fn check_output_val_size(
    tx_body: &MintedTransactionBody,
    prot_pps: &BabbageProtParams,
) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let val: &Value = match output {
            PseudoTransactionOutput::Legacy(output) => &output.amount,
            PseudoTransactionOutput::PostAlonzo(output) => &output.value,
        };
        if get_val_size_in_words(val) > prot_pps.max_val_size {
            return Err(Babbage(MaxValSizeExceeded));
        }
    }
    Ok(())
}

fn check_network_id(tx_body: &MintedTransactionBody, network_id: &u8) -> ValidationResult {
    check_tx_outs_network_id(tx_body, network_id)?;
    check_tx_network_id(tx_body, network_id)
}

fn check_tx_outs_network_id(tx_body: &MintedTransactionBody, network_id: &u8) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let addr_bytes: &Bytes = match output {
            PseudoTransactionOutput::Legacy(output) => &output.address,
            PseudoTransactionOutput::PostAlonzo(output) => &output.address,
        };
        let addr: ShelleyAddress =
            get_shelley_address(Bytes::deref(addr_bytes)).ok_or(Babbage(AddressDecoding))?;
        if addr.network().value() != *network_id {
            return Err(Babbage(OutputWrongNetworkID));
        }
    }
    Ok(())
}

// The network ID of the transaction body is either undefined or equal to the
// global network ID.
fn check_tx_network_id(tx_body: &MintedTransactionBody, network_id: &u8) -> ValidationResult {
    if let Some(tx_network_id) = tx_body.network_id {
        if get_network_id_value(tx_network_id) != *network_id {
            return Err(Babbage(TxWrongNetworkID));
        }
    }
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
