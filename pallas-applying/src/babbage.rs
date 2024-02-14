//! Utilities required for Babbage-era transaction validation.

use crate::utils::{
    add_minted_value, add_values, aux_data_from_babbage_minted_tx, compute_native_script_hash,
    compute_plutus_script_hash, compute_plutus_v2_script_hash, empty_value, get_babbage_tx_size,
    get_lovelace_from_alonzo_val, get_network_id_value, get_payment_part, get_shelley_address,
    get_val_size_in_words, lovelace_diff_or_fail, mk_alonzo_vk_wits_check_list, values_are_equal,
    verify_signature,
    BabbageError::*,
    BabbageProtParams, FeePolicy, UTxOs,
    ValidationError::{self, *},
    ValidationResult,
};
use pallas_addresses::{ScriptHash, ShelleyAddress, ShelleyPaymentPart};
use pallas_codec::utils::{Bytes, KeepRaw};
use pallas_crypto::hash::Hash;
use pallas_primitives::{
    alonzo::{RedeemerPointer, RedeemerTag},
    babbage::{
        AddrKeyhash, Mint, MintedTransactionBody, MintedTransactionOutput, MintedTx,
        MintedWitnessSet, Multiasset, NativeScript, PlutusData, PlutusV1Script, PlutusV2Script,
        PolicyId, PseudoDatumOption, PseudoTransactionOutput, Redeemer, RequiredSigners,
        TransactionInput, VKeyWitness, Value,
    },
};
use pallas_traverse::{MultiEraInput, MultiEraOutput, OriginalHash};
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
    let mut input: Value = get_consumed(tx_body, utxos)?;
    let produced: Value = get_produced(tx_body)?;
    let output: Value = add_values(
        &produced,
        &Value::Coin(tx_body.fee),
        &Babbage(NegativeValue),
    )?;
    if let Some(m) = &tx_body.mint {
        input = add_minted_value(&input, m, &Babbage(NegativeValue))?;
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

fn check_tx_size(size: &u64, prot_pps: &BabbageProtParams) -> ValidationResult {
    if *size > prot_pps.max_tx_size {
        return Err(Babbage(MaxTxSizeExceeded));
    }
    Ok(())
}

fn check_tx_ex_units(mtx: &MintedTx, prot_pps: &BabbageProtParams) -> ValidationResult {
    let tx_wits: &MintedWitnessSet = &mtx.transaction_witness_set;
    if presence_of_plutus_scripts(mtx) {
        match &tx_wits.redeemer {
            Some(redeemers_vec) => {
                let mut steps: u64 = 0;
                let mut mem: u32 = 0;
                for Redeemer { ex_units, .. } in redeemers_vec {
                    mem += ex_units.mem;
                    steps += ex_units.steps;
                }
                if mem > prot_pps.max_tx_ex_mem || steps > prot_pps.max_tx_ex_steps {
                    return Err(Babbage(TxExUnitsExceeded));
                }
            }
            None => return Err(Babbage(RedeemerMissing)),
        }
    }
    Ok(())
}

// Each minted / burned asset is paired with an appropriate native script or
// Plutus script.
fn check_minting(tx_body: &MintedTransactionBody, mtx: &MintedTx) -> ValidationResult {
    match &tx_body.mint {
        Some(minted_value) => {
            let native_script_wits: Vec<NativeScript> =
                match &mtx.transaction_witness_set.native_script {
                    None => Vec::new(),
                    Some(keep_raw_native_script_wits) => keep_raw_native_script_wits
                        .iter()
                        .map(|x| x.clone().unwrap())
                        .collect(),
                };
            let v1_script_wits: Vec<PlutusV1Script> =
                match &mtx.transaction_witness_set.plutus_v1_script {
                    None => Vec::new(),
                    Some(v1_script_wits) => v1_script_wits.clone(),
                };
            let v2_script_wits: Vec<PlutusV2Script> =
                match &mtx.transaction_witness_set.plutus_v2_script {
                    None => Vec::new(),
                    Some(v2_script_wits) => v2_script_wits.clone(),
                };
            for (policy, _) in minted_value.iter() {
                if native_script_wits
                    .iter()
                    .all(|script| compute_native_script_hash(script) != *policy)
                    && v1_script_wits
                        .iter()
                        .all(|script| compute_plutus_script_hash(script) != *policy)
                    && v2_script_wits
                        .iter()
                        .all(|script| compute_plutus_v2_script_hash(script) != *policy)
                {
                    return Err(Babbage(MintingLacksPolicy));
                }
            }
            Ok(())
        }
        None => Ok(()),
    }
}

fn check_well_formedness(_tx_body: &MintedTransactionBody, _mtx: &MintedTx) -> ValidationResult {
    Ok(())
}

fn check_witness_set(mtx: &MintedTx, utxos: &UTxOs) -> ValidationResult {
    let tx_hash: &Vec<u8> = &Vec::from(mtx.transaction_body.original_hash().as_ref());
    let tx_body: &MintedTransactionBody = &mtx.transaction_body;
    let tx_wits: &MintedWitnessSet = &mtx.transaction_witness_set;
    let vkey_wits: &Option<Vec<VKeyWitness>> = &tx_wits.vkeywitness;
    let mut native_scripts: Vec<(bool, PolicyId)> = match &tx_wits.native_script {
        Some(scripts) => scripts
            .clone()
            .iter()
            .map(|script| (false, compute_native_script_hash(script)))
            .collect(),
        None => Vec::new(),
    };
    let mut plutus_v1_scripts: Vec<(bool, PolicyId)> = match &tx_wits.plutus_v1_script {
        Some(scripts) => scripts
            .clone()
            .iter()
            .map(|script| (false, compute_plutus_script_hash(script)))
            .collect(),
        None => Vec::new(),
    };
    let mut plutus_v2_scripts: Vec<(bool, PolicyId)> = match &tx_wits.plutus_v2_script {
        Some(scripts) => scripts
            .clone()
            .iter()
            .map(|script| (false, compute_plutus_v2_script_hash(script)))
            .collect(),
        None => Vec::new(),
    };
    check_needed_scripts_except_reference_scripts(
        tx_body,
        utxos,
        &mut native_scripts,
        &mut plutus_v1_scripts,
        &mut plutus_v2_scripts,
    )?;
    check_datums(tx_body, utxos, &tx_wits.plutus_data)?;
    check_redeemers(tx_body, tx_wits, utxos)?;
    check_required_signers(&tx_body.required_signers, vkey_wits, tx_hash)?;
    check_vkey_input_wits(mtx, &tx_wits.vkeywitness, utxos)
}

// Each minting policy or script hash in a script input address can be matched
// to a script in the transaction witness set, except when it can be found in a
// reference input
fn check_needed_scripts_except_reference_scripts(
    tx_body: &MintedTransactionBody,
    utxos: &UTxOs,
    native_scripts: &mut Vec<(bool, PolicyId)>,
    plutus_v1_scripts: &mut Vec<(bool, PolicyId)>,
    plutus_v2_scripts: &mut Vec<(bool, PolicyId)>,
) -> ValidationResult {
    let reference_scripts: &Vec<ScriptHash> = &get_reference_script_hashes(tx_body, utxos);
    native_scripts.retain(|(_, native_script_hash)| {
        !reference_scripts
            .iter()
            .any(|reference_script_hash| *reference_script_hash == *native_script_hash)
    });
    plutus_v1_scripts.retain(|(_, plutus_script_v1_hash)| {
        !reference_scripts
            .iter()
            .any(|reference_script_hash| *reference_script_hash == *plutus_script_v1_hash)
    });
    plutus_v2_scripts.retain(|(_, plutus_script_v2_hash)| {
        !reference_scripts
            .iter()
            .any(|reference_script_hash| *reference_script_hash == *plutus_script_v2_hash)
    });
    check_script_inputs(
        tx_body,
        native_scripts,
        plutus_v1_scripts,
        plutus_v2_scripts,
        utxos,
    )?;
    check_minting_policies(
        tx_body,
        native_scripts,
        plutus_v1_scripts,
        plutus_v2_scripts,
    )?;
    for (native_script_covered, _) in native_scripts.iter() {
        if !native_script_covered {
            return Err(Babbage(UnneededNativeScript));
        }
    }
    for (plutus_v1_script_covered, _) in plutus_v1_scripts.iter() {
        if !plutus_v1_script_covered {
            return Err(Babbage(UnneededPlutusV1Script));
        }
    }
    for (plutus_v2_script_covered, _) in plutus_v2_scripts.iter() {
        if !plutus_v2_script_covered {
            return Err(Babbage(UnneededPlutusV2Script));
        }
    }
    Ok(())
}

fn get_reference_script_hashes(tx_body: &MintedTransactionBody, utxos: &UTxOs) -> Vec<ScriptHash> {
    let mut res: Vec<ScriptHash> = Vec::new();
    if let Some(reference_inputs) = &tx_body.reference_inputs {
        for input in reference_inputs {
            if let Some(script_hash) = get_script_hash_from_input(input, utxos) {
                res.push(script_hash)
            }
        }
    }
    res
}

fn check_script_inputs(
    tx_body: &MintedTransactionBody,
    native_scripts: &mut [(bool, PolicyId)],
    plutus_v1_scripts: &mut [(bool, PolicyId)],
    plutus_v2_scripts: &mut [(bool, PolicyId)],
    utxos: &UTxOs,
) -> ValidationResult {
    let mut inputs: Vec<(bool, ScriptHash)> = get_script_hashes(tx_body, utxos);
    for (input_script_covered, input_script_hash) in &mut inputs {
        for (native_script_covered, native_script_hash) in native_scripts.iter_mut() {
            if *input_script_hash == *native_script_hash {
                *input_script_covered = true;
                *native_script_covered = true;
            }
        }
        for (plutus_script_covered, plutus_v1_script_hash) in plutus_v1_scripts.iter_mut() {
            if *input_script_hash == *plutus_v1_script_hash {
                *input_script_covered = true;
                *plutus_script_covered = true;
            }
        }
        for (plutus_script_covered, plutus_v2_script_hash) in plutus_v2_scripts.iter_mut() {
            if *input_script_hash == *plutus_v2_script_hash {
                *input_script_covered = true;
                *plutus_script_covered = true;
            }
        }
    }
    for (input_script_covered, _) in inputs {
        if !input_script_covered {
            return Err(Babbage(ScriptWitnessMissing));
        }
    }
    Ok(())
}

fn get_script_hashes(tx_body: &MintedTransactionBody, utxos: &UTxOs) -> Vec<(bool, ScriptHash)> {
    let mut res: Vec<(bool, ScriptHash)> = Vec::new();
    for input in tx_body.inputs.iter() {
        if let Some(script_hash) = get_script_hash_from_input(input, utxos) {
            res.push((false, script_hash))
        }
    }
    res
}

fn get_script_hash_from_input(input: &TransactionInput, utxos: &UTxOs) -> Option<ScriptHash> {
    match utxos
        .get(&MultiEraInput::from_alonzo_compatible(input))
        .and_then(MultiEraOutput::as_babbage)
    {
        Some(PseudoTransactionOutput::Legacy(output)) => match get_payment_part(&output.address) {
            Some(ShelleyPaymentPart::Script(script_hash)) => Some(script_hash),
            _ => None,
        },
        Some(PseudoTransactionOutput::PostAlonzo(output)) => {
            match get_payment_part(&output.address) {
                Some(ShelleyPaymentPart::Script(script_hash)) => Some(script_hash),
                _ => None,
            }
        }
        None => None,
    }
}

fn check_minting_policies(
    tx_body: &MintedTransactionBody,
    native_scripts: &mut [(bool, PolicyId)],
    plutus_v1_scripts: &mut [(bool, PolicyId)],
    plutus_v2_scripts: &mut [(bool, PolicyId)],
) -> ValidationResult {
    match &tx_body.mint {
        None => Ok(()),
        Some(minted_value) => {
            let mut minting_policies: Vec<(bool, PolicyId)> =
                minted_value.iter().map(|(pol, _)| (false, *pol)).collect();
            for (policy_covered, policy) in &mut minting_policies {
                for (native_script_covered, native_script_hash) in native_scripts.iter_mut() {
                    if *policy == *native_script_hash {
                        *policy_covered = true;
                        *native_script_covered = true;
                    }
                }
                for (plutus_script_covered, plutus_v1_script_hash) in plutus_v1_scripts.iter_mut() {
                    if *policy == *plutus_v1_script_hash {
                        *policy_covered = true;
                        *plutus_script_covered = true;
                    }
                }
                for (plutus_script_covered, plutus_v2_script_hash) in plutus_v2_scripts.iter_mut() {
                    if *policy == *plutus_v2_script_hash {
                        *policy_covered = true;
                        *plutus_script_covered = true;
                    }
                }
            }
            for (policy_covered, _) in minting_policies {
                if !policy_covered {
                    return Err(Babbage(MintingLacksPolicy));
                }
            }
            Ok(())
        }
    }
}

// Each datum hash in a Plutus script input matches the hash of a datum in the
// transaction witness set
fn check_datums(
    tx_body: &MintedTransactionBody,
    utxos: &UTxOs,
    option_plutus_data: &Option<Vec<KeepRaw<PlutusData>>>,
) -> ValidationResult {
    let mut plutus_data_hash: Vec<(bool, Hash<32>)> = match option_plutus_data {
        Some(plutus_data) => plutus_data
            .iter()
            .map(|datum| {
                (
                    false,
                    pallas_crypto::hash::Hasher::<256>::hash(datum.raw_cbor()),
                )
            })
            .collect(),
        None => Vec::new(),
    };
    check_input_datum_hash_in_witness_set(tx_body, utxos, &mut plutus_data_hash)?;
    check_remaining_datums(&plutus_data_hash, tx_body, utxos)
}

// Each datum hash in a Plutus script input matches the hash of a datum in the
// transaction witness set.
fn check_input_datum_hash_in_witness_set(
    tx_body: &MintedTransactionBody,
    utxos: &UTxOs,
    plutus_data_hash: &mut [(bool, Hash<32>)],
) -> ValidationResult {
    for input in &tx_body.inputs {
        match utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .and_then(MultiEraOutput::as_babbage)
        {
            Some(output) => {
                if let Some(datum_hash) = get_datum_hash(output) {
                    find_datum_hash(&datum_hash, plutus_data_hash)?
                }
            }
            None => return Err(Babbage(InputNotInUTxO)),
        }
    }
    Ok(())
}

// Extract datum hash if one is contained.
fn get_datum_hash(output: &MintedTransactionOutput) -> Option<Hash<32>> {
    match output {
        PseudoTransactionOutput::Legacy(output) => output.datum_hash,
        PseudoTransactionOutput::PostAlonzo(output) => match output.datum_option {
            Some(PseudoDatumOption::Hash(hash)) => Some(hash),
            _ => None,
        },
    }
}

fn find_datum_hash(hash: &Hash<32>, plutus_data_hash: &mut [(bool, Hash<32>)]) -> ValidationResult {
    for (found, plutus_datum_hash) in plutus_data_hash {
        if hash == plutus_datum_hash {
            *found = true;
            return Ok(());
        }
    }
    Err(Babbage(DatumMissing))
}

// Each datum in the transaction witness set can be related to the datum hash in
// a Plutus script input, or in a reference input, or in a regular output, or in
// the collateral return output
fn check_remaining_datums(
    plutus_data_hash: &[(bool, Hash<32>)],
    tx_body: &MintedTransactionBody,
    utxos: &UTxOs,
) -> ValidationResult {
    for (found, plutus_datum_hash) in plutus_data_hash {
        if !found {
            find_datum(plutus_datum_hash, tx_body, utxos)?
        }
    }
    Ok(())
}

fn find_datum(hash: &Hash<32>, tx_body: &MintedTransactionBody, utxos: &UTxOs) -> ValidationResult {
    // Look for hash in transaction (regular) outputs
    for output in tx_body.outputs.iter() {
        if let Some(datum_hash) = get_datum_hash(output) {
            if *hash == datum_hash {
                return Ok(());
            }
        }
    }
    // Look for hash in collateral return output
    if let Some(babbage_output) = &tx_body.collateral_return {
        match babbage_output {
            PseudoTransactionOutput::Legacy(output) => {
                if let Some(datum_hash) = &output.datum_hash {
                    if *hash == *datum_hash {
                        return Ok(());
                    }
                }
            }
            PseudoTransactionOutput::PostAlonzo(output) => {
                if let Some(PseudoDatumOption::Hash(datum_hash)) = &output.datum_option {
                    if *hash == *datum_hash {
                        return Ok(());
                    }
                }
            }
        }
    }
    // Look for hash in reference input
    if let Some(reference_inputs) = &tx_body.reference_inputs {
        for reference_input in reference_inputs.iter() {
            match utxos
                .get(&MultiEraInput::from_alonzo_compatible(reference_input))
                .and_then(MultiEraOutput::as_babbage)
            {
                Some(PseudoTransactionOutput::Legacy(output)) => {
                    if let Some(datum_hash) = &output.datum_hash {
                        if *hash == *datum_hash {
                            return Ok(());
                        }
                    }
                }
                Some(PseudoTransactionOutput::PostAlonzo(output)) => {
                    if let Some(PseudoDatumOption::Hash(datum_hash)) = &output.datum_option {
                        if *hash == *datum_hash {
                            return Ok(());
                        }
                    }
                }
                _ => (),
            }
        }
    }
    Err(Babbage(UnneededDatum))
}

fn check_redeemers(
    tx_body: &MintedTransactionBody,
    tx_wits: &MintedWitnessSet,
    utxos: &UTxOs,
) -> ValidationResult {
    let _redeemer_pointers: Vec<RedeemerPointer> = match &tx_wits.redeemer {
        Some(redeemers) => redeemers
            .iter()
            .map(|x| RedeemerPointer {
                tag: x.tag.clone(),
                index: x.index,
            })
            .collect(),
        None => Vec::new(),
    };
    let _plutus_scripts: Vec<RedeemerPointer> = mk_plutus_script_redeemer_pointers(
        &sort_inputs(&tx_body.inputs),
        &tx_body.mint,
        tx_wits,
        utxos,
    );
    Ok(())
}

// Lexicographical sorting for inputs.
fn sort_inputs(unsorted_inputs: &[TransactionInput]) -> Vec<TransactionInput> {
    let mut res: Vec<TransactionInput> = unsorted_inputs.to_owned();
    res.sort();
    res
}

fn mk_plutus_script_redeemer_pointers(
    sorted_inputs: &[TransactionInput],
    mint: &Option<Multiasset<i64>>,
    tx_wits: &MintedWitnessSet,
    utxos: &UTxOs,
) -> Vec<RedeemerPointer> {
    let mut res: Vec<RedeemerPointer> = Vec::new();
    if let Some(plutus_v1_scripts) = &tx_wits.plutus_v1_script {
        for (index, input) in sorted_inputs.iter().enumerate() {
            if let Some(script_hash) = get_script_hash_from_input(input, utxos) {
                for plutus_script in plutus_v1_scripts.iter() {
                    let hashed_script: PolicyId = compute_plutus_script_hash(plutus_script);
                    if script_hash == hashed_script {
                        res.push(RedeemerPointer {
                            tag: RedeemerTag::Spend,
                            index: index as u32,
                        })
                    }
                }
            }
        }
        match mint {
            Some(minted_value) => {
                for (index, policy) in sort_policies(minted_value).iter().enumerate() {
                    for plutus_v1_script in plutus_v1_scripts.iter() {
                        if *policy == compute_plutus_script_hash(plutus_v1_script) {
                            res.push(RedeemerPointer {
                                tag: RedeemerTag::Mint,
                                index: index as u32,
                            })
                        }
                    }
                }
            }
            None => (),
        }
    }
    if let Some(plutus_v2_scripts) = &tx_wits.plutus_v2_script {
        for (index, input) in sorted_inputs.iter().enumerate() {
            if let Some(script_hash) = get_script_hash_from_input(input, utxos) {
                for plutus_v2_script in plutus_v2_scripts.iter() {
                    let hashed_script: PolicyId = compute_plutus_v2_script_hash(plutus_v2_script);
                    if script_hash == hashed_script {
                        res.push(RedeemerPointer {
                            tag: RedeemerTag::Spend,
                            index: index as u32,
                        })
                    }
                }
            }
        }
        match mint {
            Some(minted_value) => {
                for (index, policy) in sort_policies(minted_value).iter().enumerate() {
                    for plutus_v2_script in plutus_v2_scripts.iter() {
                        if *policy == compute_plutus_v2_script_hash(plutus_v2_script) {
                            res.push(RedeemerPointer {
                                tag: RedeemerTag::Mint,
                                index: index as u32,
                            })
                        }
                    }
                }
            }
            None => (),
        }
    }
    res
}

// Lexicographical sorting for PolicyID's.
fn sort_policies(mint: &Mint) -> Vec<PolicyId> {
    let mut res: Vec<PolicyId> = mint
        .clone()
        .to_vec()
        .iter()
        .map(|(policy_id, _)| *policy_id)
        .collect();
    res.sort();
    res
}

// All required signers (needed by a Plutus script) have a corresponding match
// in the transaction witness set.
fn check_required_signers(
    required_signers: &Option<RequiredSigners>,
    vkey_wits: &Option<Vec<VKeyWitness>>,
    data_to_verify: &[u8],
) -> ValidationResult {
    if let Some(req_signers) = &required_signers {
        match &vkey_wits {
            Some(vkey_wits) => {
                for req_signer in req_signers {
                    find_and_check_req_signer(req_signer, vkey_wits, data_to_verify)?
                }
            }
            None => return Err(Babbage(ReqSignerMissing)),
        }
    }
    Ok(())
}

// Try to find the verification key in the witnesses, and verify the signature.
fn find_and_check_req_signer(
    vkey_hash: &AddrKeyhash,
    vkey_wits: &[VKeyWitness],
    data_to_verify: &[u8],
) -> ValidationResult {
    for vkey_wit in vkey_wits {
        if pallas_crypto::hash::Hasher::<224>::hash(&vkey_wit.vkey.clone()) == *vkey_hash {
            if !verify_signature(vkey_wit, data_to_verify) {
                return Err(Babbage(ReqSignerWrongSig));
            } else {
                return Ok(());
            }
        }
    }
    Err(Babbage(ReqSignerMissing))
}

fn check_vkey_input_wits(
    mtx: &MintedTx,
    vkey_wits: &Option<Vec<VKeyWitness>>,
    utxos: &UTxOs,
) -> ValidationResult {
    let tx_body: &MintedTransactionBody = &mtx.transaction_body;
    let vk_wits: &mut Vec<(bool, VKeyWitness)> =
        &mut mk_alonzo_vk_wits_check_list(vkey_wits, Babbage(VKWitnessMissing))?;
    let tx_hash: &Vec<u8> = &Vec::from(mtx.transaction_body.original_hash().as_ref());
    let mut inputs_and_collaterals: Vec<TransactionInput> = Vec::new();
    inputs_and_collaterals.extend(tx_body.inputs.clone());
    if let Some(collaterals) = &tx_body.collateral {
        inputs_and_collaterals.extend(collaterals.clone())
    }
    for input in inputs_and_collaterals.iter() {
        match utxos.get(&MultiEraInput::from_alonzo_compatible(input)) {
            Some(multi_era_output) => {
                if let Some(babbage_output) = MultiEraOutput::as_babbage(multi_era_output) {
                    let address: &Bytes = match babbage_output {
                        PseudoTransactionOutput::Legacy(output) => &output.address,
                        PseudoTransactionOutput::PostAlonzo(output) => &output.address,
                    };
                    match get_payment_part(address).ok_or(Babbage(InputDecoding))? {
                        ShelleyPaymentPart::Key(payment_key_hash) => {
                            check_vk_wit(&payment_key_hash, vk_wits, tx_hash)?
                        }
                        ShelleyPaymentPart::Script(_) => (),
                    }
                }
            }
            None => return Err(Babbage(InputNotInUTxO)),
        }
    }
    check_remaining_vk_wits(vk_wits, tx_hash) // required for native scripts
}

fn check_vk_wit(
    payment_key_hash: &AddrKeyhash,
    wits: &mut [(bool, VKeyWitness)],
    data_to_verify: &[u8],
) -> ValidationResult {
    for (vkey_wit_covered, vkey_wit) in wits {
        if pallas_crypto::hash::Hasher::<224>::hash(&vkey_wit.vkey.clone()) == *payment_key_hash {
            if !verify_signature(vkey_wit, data_to_verify) {
                return Err(Babbage(VKWrongSignature));
            } else {
                *vkey_wit_covered = true;
                return Ok(());
            }
        }
    }
    Err(Babbage(VKWitnessMissing))
}

fn check_remaining_vk_wits(
    wits: &mut [(bool, VKeyWitness)],
    data_to_verify: &[u8],
) -> ValidationResult {
    for (covered, vkey_wit) in wits {
        if !*covered {
            if verify_signature(vkey_wit, data_to_verify) {
                return Ok(());
            } else {
                return Err(Babbage(VKWrongSignature));
            }
        }
    }
    Ok(())
}

fn check_languages(_mtx: &MintedTx, _prot_pps: &BabbageProtParams) -> ValidationResult {
    Ok(())
}

// The metadata of the transaction is valid.
fn check_auxiliary_data(tx_body: &MintedTransactionBody, mtx: &MintedTx) -> ValidationResult {
    match (
        &tx_body.auxiliary_data_hash,
        aux_data_from_babbage_minted_tx(mtx),
    ) {
        (Some(metadata_hash), Some(metadata)) => {
            if metadata_hash.as_slice()
                == pallas_crypto::hash::Hasher::<256>::hash(metadata).as_ref()
            {
                Ok(())
            } else {
                Err(Babbage(MetadataHash))
            }
        }
        (None, None) => Ok(()),
        _ => Err(Babbage(MetadataHash)),
    }
}

fn check_script_data_hash(_tx_body: &MintedTransactionBody, _mtx: &MintedTx) -> ValidationResult {
    Ok(())
}
