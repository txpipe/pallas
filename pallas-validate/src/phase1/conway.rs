//! Utilities required for Conway-era transaction validation.

use crate::utils::{
    aux_data_from_conway_tx, compute_native_script_hash, compute_plutus_v1_script_hash,
    compute_plutus_v2_script_hash, compute_plutus_v3_script_hash, conway_add_minted_non_zero,
    conway_add_values, conway_get_val_size_in_words, conway_lovelace_diff_or_fail,
    conway_values_are_equal, get_conway_tx_size, get_lovelace_from_conway_val, get_payment_part,
    get_shelley_address, is_byron_address, mk_alonzo_vk_wits_check_list, verify_signature,
    ConwayProtParams,
    PostAlonzoError::*,
    UTxOs,
    ValidationError::{self, *},
    ValidationResult,
};
use pallas_addresses::{Address, ScriptHash, ShelleyAddress, ShelleyPaymentPart};
use pallas_codec::utils::{Bytes, KeepRaw};
use pallas_primitives::{
    babbage,
    conway::{
        DatumOption, Language, LanguageView, Mint, Redeemers, RedeemersKey, RequiredSigners,
        ScriptRef, TransactionBody, TransactionOutput, Tx, VKeyWitness, Value, WitnessSet,
    },
    AddrKeyhash, Hash, PlutusData, PlutusScript, PolicyId, PositiveCoin, TransactionInput,
};
use pallas_traverse::{MultiEraInput, MultiEraOutput, OriginalHash};
use std::ops::Deref;

pub fn validate_conway_tx(
    mtx: &Tx,
    utxos: &UTxOs,
    prot_pps: &ConwayProtParams,
    block_slot: &u64,
    network_id: &u8,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body.clone();
    let size: u32 = get_conway_tx_size(mtx).ok_or(PostAlonzo(UnknownTxSize))?;
    check_ins_not_empty(tx_body)?;
    check_all_ins_in_utxos(tx_body, utxos)?;
    check_tx_validity_interval(tx_body, block_slot)?;
    check_fee(tx_body, &size, mtx, utxos, prot_pps)?;
    check_preservation_of_value(tx_body, utxos)?;
    check_min_lovelace(tx_body, prot_pps)?;
    check_output_val_size(tx_body, prot_pps)?;
    check_network_id(tx_body, network_id)?;
    check_tx_size(&size, prot_pps)?;
    check_tx_ex_units(mtx, prot_pps)?;
    check_minting(tx_body, mtx, utxos)?;
    check_well_formedness(tx_body, mtx)?;
    check_witness_set(mtx, utxos)?;
    check_languages(mtx, utxos, prot_pps)?;
    check_auxiliary_data(tx_body, mtx)?;
    check_script_data_hash(tx_body, mtx, utxos, prot_pps)
}

// The set of transaction inputs is not empty.
fn check_ins_not_empty(tx_body: &TransactionBody) -> ValidationResult {
    if tx_body.inputs.is_empty() {
        return Err(PostAlonzo(TxInsEmpty));
    }
    Ok(())
}

// All transaction inputs, collateral inputs and reference inputs are in the
// UTxO set.
fn check_all_ins_in_utxos(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    for input in tx_body.inputs.iter() {
        if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(input))) {
            return Err(PostAlonzo(InputNotInUTxO));
        }
    }
    match &tx_body.collateral {
        None => (),
        Some(collaterals) => {
            for collateral in collaterals {
                if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(collateral))) {
                    return Err(PostAlonzo(CollateralNotInUTxO));
                }
            }
        }
    }
    match &tx_body.reference_inputs {
        None => (),
        Some(reference_inputs) => {
            for reference_input in reference_inputs {
                if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(reference_input))) {
                    return Err(PostAlonzo(ReferenceInputNotInUTxO));
                }
            }
        }
    }
    Ok(())
}

// The block slot is contained in the transaction validity interval, and the
// upper bound is translatable to UTC time.
fn check_tx_validity_interval(tx_body: &TransactionBody, block_slot: &u64) -> ValidationResult {
    check_lower_bound(tx_body, *block_slot)?;
    check_upper_bound(tx_body, *block_slot)
}

// If defined, the lower bound of the validity time interval does not exceed the
// block slot.
fn check_lower_bound(tx_body: &TransactionBody, block_slot: u64) -> ValidationResult {
    match tx_body.validity_interval_start {
        Some(lower_bound) => {
            if block_slot < lower_bound {
                Err(PostAlonzo(BlockPrecedesValInt))
            } else {
                Ok(())
            }
        }
        None => Ok(()),
    }
}

// If defined, the upper bound of the validity time interval is not exceeded by
// the block slot, and it is translatable to UTC time.
fn check_upper_bound(tx_body: &TransactionBody, block_slot: u64) -> ValidationResult {
    match tx_body.ttl {
        Some(upper_bound) => {
            if upper_bound < block_slot {
                Err(PostAlonzo(BlockExceedsValInt))
            } else {
                // TODO: check that `upper_bound` is translatable to UTC time.
                Ok(())
            }
        }
        None => Ok(()),
    }
}

fn check_fee(
    tx_body: &TransactionBody,
    size: &u32,
    mtx: &Tx,
    utxos: &UTxOs,
    prot_pps: &ConwayProtParams,
) -> ValidationResult {
    check_min_fee(tx_body, size, prot_pps)?;
    if presence_of_plutus_scripts(mtx) {
        check_collaterals(tx_body, utxos, prot_pps)?
    }
    Ok(())
}

// The fee paid by the transaction should be greater than or equal to the
// minimum fee.
fn check_min_fee(
    tx_body: &TransactionBody,
    size: &u32,
    prot_pps: &ConwayProtParams,
) -> ValidationResult {
    if tx_body.fee < (prot_pps.minfee_b + prot_pps.minfee_a * size) as u64 {
        return Err(PostAlonzo(FeeBelowMin));
    }
    Ok(())
}

fn presence_of_plutus_scripts(mtx: &Tx) -> bool {
    let minted_witness_set: &WitnessSet = &mtx.transaction_witness_set;
    let plutus_v1_scripts: &[PlutusScript<1>] = minted_witness_set
        .plutus_v1_script
        .as_ref()
        .map(|x| x.as_slice())
        .unwrap_or(&[]);
    let plutus_v2_scripts: &[PlutusScript<2>] = minted_witness_set
        .plutus_v2_script
        .as_ref()
        .map(|x| x.as_slice())
        .unwrap_or(&[]);
    let plutus_v3_scripts: &[PlutusScript<3>] = minted_witness_set
        .plutus_v3_script
        .as_ref()
        .map(|x| x.as_slice())
        .unwrap_or(&[]);
    !plutus_v1_scripts.is_empty() || !plutus_v2_scripts.is_empty() || !plutus_v3_scripts.is_empty()
}

fn check_collaterals(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    prot_pps: &ConwayProtParams,
) -> ValidationResult {
    let collaterals: &[TransactionInput] = &tx_body
        .collateral
        .clone()
        .ok_or(PostAlonzo(CollateralMissing))?;
    check_collaterals_number(collaterals, prot_pps)?;
    check_collaterals_address(collaterals, utxos)?;
    check_collaterals_assets(tx_body, utxos, prot_pps)
}

// The set of collateral inputs is not empty.
// The number of collateral inputs is below maximum allowed by protocol.
fn check_collaterals_number(
    collaterals: &[TransactionInput],
    prot_pps: &ConwayProtParams,
) -> ValidationResult {
    if collaterals.is_empty() {
        Err(PostAlonzo(CollateralMissing))
    } else if collaterals.len() as u32 > prot_pps.max_collateral_inputs {
        Err(PostAlonzo(TooManyCollaterals))
    } else {
        Ok(())
    }
}

// Each collateral input refers to a verification-key address.
fn check_collaterals_address(collaterals: &[TransactionInput], utxos: &UTxOs) -> ValidationResult {
    for collateral in collaterals {
        match utxos.get(&MultiEraInput::from_alonzo_compatible(collateral)) {
            Some(multi_era_output) => {
                if let Some(conway_output) = MultiEraOutput::as_conway(multi_era_output) {
                    let address: &Bytes = match conway_output {
                        TransactionOutput::Legacy(inner) => &inner.address,
                        TransactionOutput::PostAlonzo(inner) => &inner.address,
                    };
                    if let ShelleyPaymentPart::Script(_) =
                        get_payment_part(address).ok_or(PostAlonzo(InputDecoding))?
                    {
                        return Err(PostAlonzo(CollateralNotVKeyLocked));
                    }
                }
            }
            None => {
                return Err(PostAlonzo(CollateralNotInUTxO));
            }
        }
    }
    Ok(())
}

// The balance between collateral inputs and output contains only lovelace.
// The balance is not lower than the minimum allowed.
// The balance matches exactly the collateral annotated in the transaction body.
fn check_collaterals_assets(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    prot_pps: &ConwayProtParams,
) -> ValidationResult {
    match &tx_body.collateral {
        Some(collaterals) => {
            let first_collateral = collaterals.first().unwrap();
            let mut coll_input =
                match utxos.get(&MultiEraInput::from_alonzo_compatible(first_collateral)) {
                    Some(multi_era_output) => val_from_multi_era_output(multi_era_output),
                    None => return Err(PostAlonzo(CollateralNotInUTxO)),
                };
            for collateral in collaterals.iter().skip(1) {
                match utxos.get(&MultiEraInput::from_alonzo_compatible(collateral)) {
                    Some(multi_era_output) => {
                        coll_input = conway_add_values(
                            &coll_input,
                            &val_from_multi_era_output(multi_era_output),
                            &PostAlonzo(NegativeValue),
                        )?
                    }
                    None => {
                        return Err(PostAlonzo(CollateralNotInUTxO));
                    }
                }
            }
            let coll_return: Value = match &tx_body.collateral_return {
                Some(TransactionOutput::Legacy(output)) => {
                    let amount = output.amount.clone();
                    match amount {
                        babbage::Value::Coin(coin) => Value::Coin(coin),
                        babbage::Value::Multiasset(coin, assets) => {
                            let mut conway_assets = Vec::new();
                            for (key, val) in assets.into_iter() {
                                let mut conway_value = Vec::new();
                                for (inner_key, inner_val) in val.into_iter() {
                                    conway_value.push((
                                        inner_key,
                                        PositiveCoin::try_from(inner_val).unwrap(),
                                    ));
                                }
                                conway_assets.push((key, conway_value.into_iter().collect()));
                            }
                            let conway_assets = conway_assets.into_iter().collect();
                            Value::Multiasset(coin, conway_assets)
                        }
                    }
                }
                Some(TransactionOutput::PostAlonzo(output)) => output.value.clone(),
                None => Value::Coin(0),
            };
            // The balance between collateral inputs and output contains only lovelace.
            let paid_collateral: u64 = conway_lovelace_diff_or_fail(
                &coll_input,
                &coll_return,
                &PostAlonzo(NonLovelaceCollateral),
            )?;
            let fee_percentage: u64 = tx_body.fee * prot_pps.collateral_percentage as u64;
            // The balance is not lower than the minimum allowed.
            if paid_collateral * 100 < fee_percentage {
                return Err(PostAlonzo(CollateralMinLovelace));
            }
            // The balance matches exactly the collateral annotated in the transaction body.
            if let Some(annotated_collateral) = &tx_body.total_collateral {
                if paid_collateral != *annotated_collateral {
                    return Err(PostAlonzo(CollateralAnnotation));
                }
            }
        }
        None => return Err(PostAlonzo(CollateralMissing)),
    }
    Ok(())
}

fn val_from_multi_era_output(multi_era_output: &MultiEraOutput) -> Value {
    let value = multi_era_output.value();
    value.into_conway()
}

// The preservation of value property holds.
fn check_preservation_of_value(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    let mut input: Value = get_consumed(tx_body, utxos)?;
    let produced: Value = get_produced(tx_body)?;
    let output: Value = conway_add_values(
        &produced,
        &Value::Coin(tx_body.fee),
        &PostAlonzo(NegativeValue),
    )?;
    if let Some(m) = &tx_body.mint {
        input = conway_add_minted_non_zero(&input, m, &PostAlonzo(NegativeValue))?;
    }

    if !conway_values_are_equal(&input, &output) {
        return Err(PostAlonzo(PreservationOfValue));
    }
    Ok(())
}

fn get_consumed(tx_body: &TransactionBody, utxos: &UTxOs) -> Result<Value, ValidationError> {
    let mut inputs_iter = tx_body.inputs.iter();
    let Some(first_input) = inputs_iter.next() else {
        return Err(PostAlonzo(TxInsEmpty));
    };
    let multi_era_output: &MultiEraOutput = utxos
        .get(&MultiEraInput::from_alonzo_compatible(first_input))
        .ok_or(PostAlonzo(InputNotInUTxO))?;
    let mut res: Value = val_from_multi_era_output(multi_era_output);
    for input in inputs_iter {
        let multi_era_output: &MultiEraOutput = utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .ok_or(PostAlonzo(InputNotInUTxO))?;
        let val: Value = val_from_multi_era_output(multi_era_output);
        res = conway_add_values(&res, &val, &PostAlonzo(NegativeValue))?;
    }
    Ok(res)
}

fn get_produced(tx_body: &TransactionBody) -> Result<Value, ValidationError> {
    let mut outputs_iter = tx_body.outputs.iter();
    let Some(first_output) = outputs_iter.next() else {
        return Err(PostAlonzo(TxInsEmpty));
    };
    let mut res: Value = match first_output {
        TransactionOutput::Legacy(output) => {
            let amount = output.amount.clone();
            match amount {
                babbage::Value::Coin(coin) => Value::Coin(coin),
                babbage::Value::Multiasset(coin, assets) => {
                    let mut conway_assets = Vec::new();
                    for (key, val) in assets.into_iter() {
                        let mut conway_value = Vec::new();
                        for (inner_key, inner_val) in val.into_iter() {
                            conway_value
                                .push((inner_key, PositiveCoin::try_from(inner_val).unwrap()));
                        }
                        conway_assets.push((key, conway_value.into_iter().collect()));
                    }
                    let conway_assets = conway_assets.into_iter().collect();
                    Value::Multiasset(coin, conway_assets)
                }
            }
        }
        TransactionOutput::PostAlonzo(output) => output.value.clone(),
    };
    for output in outputs_iter {
        match output {
            TransactionOutput::Legacy(output) => {
                let amount = output.amount.clone();
                match amount {
                    babbage::Value::Coin(coin) => {
                        res =
                            conway_add_values(&res, &Value::Coin(coin), &PostAlonzo(NegativeValue))?
                    }
                    babbage::Value::Multiasset(coin, assets) => {
                        let mut conway_assets = Vec::new();
                        for (key, val) in assets.into_iter() {
                            let mut conway_value = Vec::new();
                            for (inner_key, inner_val) in val.into_iter() {
                                conway_value
                                    .push((inner_key, PositiveCoin::try_from(inner_val).unwrap()));
                            }
                            conway_assets.push((key, conway_value.into_iter().collect()));
                        }
                        let conway_assets = conway_assets.into_iter().collect();
                        res = conway_add_values(
                            &res,
                            &Value::Multiasset(coin, conway_assets),
                            &PostAlonzo(NegativeValue),
                        )?
                    }
                }
            }
            TransactionOutput::PostAlonzo(output) => {
                res = conway_add_values(&res, &output.value, &PostAlonzo(NegativeValue))?
            }
        }
    }
    Ok(res)
}

fn check_min_lovelace(tx_body: &TransactionBody, prot_pps: &ConwayProtParams) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let val: &Value = match output {
            TransactionOutput::Legacy(output) => {
                let amount = output.amount.clone();
                match amount {
                    babbage::Value::Coin(coin) => &Value::Coin(coin),
                    babbage::Value::Multiasset(coin, assets) => {
                        let mut conway_assets = Vec::new();
                        for (key, val) in assets.into_iter() {
                            let mut conway_value = Vec::new();
                            for (inner_key, inner_val) in val.into_iter() {
                                conway_value
                                    .push((inner_key, PositiveCoin::try_from(inner_val).unwrap()));
                            }
                            conway_assets.push((key, conway_value.into_iter().collect()));
                        }
                        let conway_assets = conway_assets.into_iter().collect();
                        &Value::Multiasset(coin, conway_assets)
                    }
                }
            }
            TransactionOutput::PostAlonzo(output) => &output.value,
        };
        if get_lovelace_from_conway_val(val) < compute_min_lovelace(val, prot_pps) {
            return Err(PostAlonzo(MinLovelaceUnreached));
        }
    }
    Ok(())
}

fn compute_min_lovelace(val: &Value, prot_pps: &ConwayProtParams) -> u64 {
    prot_pps.ada_per_utxo_byte * (conway_get_val_size_in_words(val) + 160)
}

// The size of the value in each of the outputs should not be greater than the
// maximum allowed.
fn check_output_val_size(
    tx_body: &TransactionBody,
    prot_pps: &ConwayProtParams,
) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let val: &Value = match output {
            TransactionOutput::Legacy(output) => {
                let amount = output.amount.clone();
                match amount {
                    babbage::Value::Coin(coin) => &Value::Coin(coin),
                    babbage::Value::Multiasset(coin, assets) => {
                        let mut conway_assets = Vec::new();
                        for (key, val) in assets.into_iter() {
                            let mut conway_value = Vec::new();
                            for (inner_key, inner_val) in val.into_iter() {
                                conway_value
                                    .push((inner_key, PositiveCoin::try_from(inner_val).unwrap()));
                            }
                            conway_assets.push((key, conway_value.into_iter().collect()));
                        }
                        let conway_assets = conway_assets.into_iter().collect();
                        &Value::Multiasset(coin, conway_assets)
                    }
                }
            }
            TransactionOutput::PostAlonzo(output) => &output.value,
        };
        if conway_get_val_size_in_words(val) > prot_pps.max_value_size as u64 {
            return Err(PostAlonzo(MaxValSizeExceeded));
        }
    }
    Ok(())
}

fn check_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    check_tx_outs_network_id(tx_body, network_id)?;
    check_tx_network_id(tx_body, network_id)
}

fn check_tx_outs_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let addr_bytes: &Bytes = match output {
            TransactionOutput::Legacy(output) => &output.address,
            TransactionOutput::PostAlonzo(output) => &output.address,
        };
        let addr: ShelleyAddress =
            get_shelley_address(Bytes::deref(addr_bytes)).ok_or(PostAlonzo(AddressDecoding))?;
        if addr.network().value() != *network_id {
            return Err(PostAlonzo(OutputWrongNetworkID));
        }
    }
    Ok(())
}

// The network ID of the transaction body is either undefined or equal to the
// global network ID.
fn check_tx_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    if let Some(tx_network_id) = tx_body.network_id {
        if u8::from(tx_network_id) != *network_id {
            return Err(PostAlonzo(TxWrongNetworkID));
        }
    }
    Ok(())
}

fn check_tx_size(size: &u32, prot_pps: &ConwayProtParams) -> ValidationResult {
    if *size > prot_pps.max_transaction_size {
        return Err(PostAlonzo(MaxTxSizeExceeded));
    }
    Ok(())
}

fn check_tx_ex_units(mtx: &Tx, prot_pps: &ConwayProtParams) -> ValidationResult {
    let tx_wits: &WitnessSet = &mtx.transaction_witness_set;
    if presence_of_plutus_scripts(mtx) {
        match &tx_wits.redeemer {
            Some(redeemers_vec) => {
                let mut steps: u64 = 0;
                let mut mem: u64 = 0;
                match redeemers_vec.clone().unwrap() {
                    Redeemers::List(r) => {
                        let _ = r.iter().map(|x| {
                            mem += x.ex_units.mem;
                            steps += x.ex_units.steps;
                        });
                    }
                    Redeemers::Map(r) => {
                        let _ = r.iter().map(|x| {
                            mem += x.1.ex_units.mem;
                            steps += x.1.ex_units.steps;
                        });
                    }
                }

                if mem > prot_pps.max_tx_ex_units.mem || steps > prot_pps.max_tx_ex_units.steps {
                    return Err(PostAlonzo(TxExUnitsExceeded));
                }
            }
            None => return Err(PostAlonzo(RedeemerMissing)),
        }
    }
    Ok(())
}

// Each minted / burned asset is paired with an appropriate native script or
// Plutus script.
fn check_minting(tx_body: &TransactionBody, mtx: &Tx, utxos: &UTxOs) -> ValidationResult {
    match &tx_body.mint {
        Some(minted_value) => {
            let native_script_wits = mtx
                .transaction_witness_set
                .native_script
                .iter()
                .flatten()
                .map(|x| compute_native_script_hash(x));

            let v1_scripts_wits = mtx
                .transaction_witness_set
                .plutus_v1_script
                .iter()
                .flatten()
                .map(compute_plutus_v1_script_hash);

            let v2_scripts_wits = mtx
                .transaction_witness_set
                .plutus_v2_script
                .iter()
                .flatten()
                .map(compute_plutus_v2_script_hash);

            let v3_scripts_wits = mtx
                .transaction_witness_set
                .plutus_v3_script
                .iter()
                .flatten()
                .map(compute_plutus_v3_script_hash);

            let ref_scripts = tx_body
                .reference_inputs
                .iter()
                .flatten()
                .filter_map(|x| get_script_hash_from_reference_input(x, utxos));

            let all_scripts_wits: Vec<_> = native_script_wits
                .chain(v1_scripts_wits)
                .chain(v2_scripts_wits)
                .chain(v3_scripts_wits)
                .chain(ref_scripts)
                .collect();

            for (policy, _) in minted_value.iter() {
                if !all_scripts_wits.contains(policy) {
                    return Err(PostAlonzo(MintingLacksPolicy(*policy)));
                }
            }
            Ok(())
        }
        None => Ok(()),
    }
}

fn check_well_formedness(_tx_body: &TransactionBody, _mtx: &Tx) -> ValidationResult {
    Ok(())
}

fn check_witness_set(mtx: &Tx, utxos: &UTxOs) -> ValidationResult {
    let tx_hash: &Vec<u8> = &Vec::from(mtx.transaction_body.original_hash().as_ref());
    let tx_body: &TransactionBody = &mtx.transaction_body;
    let tx_wits: &WitnessSet = &mtx.transaction_witness_set;
    let vkey_wits: &Option<Vec<VKeyWitness>> = &Some(
        tx_wits
            .vkeywitness
            .clone()
            .map(|wits| wits.to_vec())
            .unwrap_or_default(),
    );
    let native_scripts: Vec<PolicyId> = match &tx_wits.native_script {
        Some(scripts) => scripts
            .clone()
            .iter()
            .map(|raw_script| compute_native_script_hash(raw_script))
            .collect(),
        None => Vec::new(),
    };
    let plutus_v1_scripts: Vec<PolicyId> = match &tx_wits.plutus_v1_script {
        Some(scripts) => scripts
            .clone()
            .iter()
            .map(compute_plutus_v1_script_hash)
            .collect(),
        None => Vec::new(),
    };
    let plutus_v2_scripts: Vec<PolicyId> = match &tx_wits.plutus_v2_script {
        Some(scripts) => scripts
            .clone()
            .iter()
            .map(compute_plutus_v2_script_hash)
            .collect(),
        None => Vec::new(),
    };
    let plutus_v3_scripts: Vec<PolicyId> = match &tx_wits.plutus_v3_script {
        Some(scripts) => scripts
            .clone()
            .iter()
            .map(compute_plutus_v3_script_hash)
            .collect(),
        None => Vec::new(),
    };
    let reference_scripts: Vec<PolicyId> = get_reference_script_hashes(tx_body, utxos);
    check_needed_scripts(
        tx_body,
        utxos,
        &native_scripts,
        &plutus_v1_scripts,
        &plutus_v2_scripts,
        &plutus_v3_scripts,
        &reference_scripts,
    )?;
    let plutus_data = tx_wits.plutus_data.clone().map(|data| data.to_vec());
    check_datums(tx_body, utxos, &plutus_data)?;
    check_redeemers(
        &plutus_v1_scripts,
        &plutus_v2_scripts,
        &plutus_v3_scripts,
        &reference_scripts,
        tx_body,
        tx_wits,
        utxos,
    )?;
    check_required_signers(&tx_body.required_signers, vkey_wits, tx_hash)?;
    check_vkey_input_wits(
        mtx,
        &Some(
            tx_wits
                .vkeywitness
                .clone()
                .map(|wits| wits.to_vec())
                .unwrap_or_default(),
        ),
        utxos,
    )
}

// Each minting policy or script hash in a script input address can be matched
// to a script in the transaction witness set, except when it can be found in a
// reference input
fn check_needed_scripts(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    native_scripts: &[PolicyId],
    plutus_v1_scripts: &[PolicyId],
    plutus_v2_scripts: &[PolicyId],
    plutus_v3_scripts: &[PolicyId],
    reference_scripts: &[PolicyId],
) -> ValidationResult {
    let mut filtered_native_scripts: Vec<(bool, PolicyId)> = native_scripts
        .iter()
        .map(|&script_hash| (false, script_hash))
        .collect();
    filtered_native_scripts
        .retain(|&(_, native_script_hash)| !reference_scripts.contains(&native_script_hash));
    let mut filtered_plutus_v1_scripts: Vec<(bool, PolicyId)> = plutus_v1_scripts
        .iter()
        .map(|&script_hash| (false, script_hash))
        .collect();
    filtered_plutus_v1_scripts
        .retain(|&(_, plutus_v1_script_hash)| !reference_scripts.contains(&plutus_v1_script_hash));
    let mut filtered_plutus_v2_scripts: Vec<(bool, PolicyId)> = plutus_v2_scripts
        .iter()
        .map(|&script_hash| (false, script_hash))
        .collect();
    filtered_plutus_v2_scripts
        .retain(|&(_, plutus_v2_script_hash)| !reference_scripts.contains(&plutus_v2_script_hash));
    let mut filtered_plutus_v3_scripts: Vec<(bool, PolicyId)> = plutus_v3_scripts
        .iter()
        .map(|&script_hash| (false, script_hash))
        .collect();
    filtered_plutus_v3_scripts
        .retain(|&(_, plutus_v3_script_hash)| !reference_scripts.contains(&plutus_v3_script_hash));
    check_input_scripts(
        tx_body,
        &mut filtered_native_scripts,
        &mut filtered_plutus_v1_scripts,
        &mut filtered_plutus_v2_scripts,
        &mut filtered_plutus_v3_scripts,
        reference_scripts,
        utxos,
    )?;
    check_minting_policies(
        tx_body,
        &mut filtered_native_scripts,
        &mut filtered_plutus_v1_scripts,
        &mut filtered_plutus_v2_scripts,
        &mut filtered_plutus_v3_scripts,
        reference_scripts,
    )?;
    for (covered, _) in filtered_native_scripts.iter() {
        if !covered {
            return Err(PostAlonzo(UnneededNativeScript));
        }
    }
    for (covered, _) in filtered_plutus_v1_scripts.iter() {
        if !covered {
            return Err(PostAlonzo(UnneededPlutusV1Script));
        }
    }
    for (covered, _) in filtered_plutus_v2_scripts.iter() {
        if !covered {
            return Err(PostAlonzo(UnneededPlutusV2Script));
        }
    }
    for (covered, _) in filtered_plutus_v3_scripts.iter() {
        if !covered {
            return Err(PostAlonzo(UnneededPlutusV2Script));
        }
    }
    Ok(())
}

fn get_reference_script_hashes(tx_body: &TransactionBody, utxos: &UTxOs) -> Vec<PolicyId> {
    let mut res: Vec<PolicyId> = Vec::new();
    if let Some(reference_inputs) = &tx_body.reference_inputs {
        for input in reference_inputs.iter() {
            if let Some(script_hash) = get_script_hash_from_reference_input(input, utxos) {
                res.push(script_hash)
            }
        }
    }
    res
}

fn check_input_scripts(
    tx_body: &TransactionBody,
    native_scripts: &mut [(bool, PolicyId)],
    plutus_v1_scripts: &mut [(bool, PolicyId)],
    plutus_v2_scripts: &mut [(bool, PolicyId)],
    pluts_v3_scripts: &mut [(bool, PolicyId)],
    reference_scripts: &[PolicyId],
    utxos: &UTxOs,
) -> ValidationResult {
    let mut needed_input_scripts: Vec<(bool, ScriptHash)> =
        get_script_hashes_from_inputs(tx_body, utxos);
    for (covered, hash) in &mut needed_input_scripts {
        for (native_script_covered, native_script_hash) in native_scripts.iter_mut() {
            if *hash == *native_script_hash {
                *covered = true;
                *native_script_covered = true;
            }
        }
        for (plutus_v1_script_covered, plutus_v1_script_hash) in plutus_v1_scripts.iter_mut() {
            if *hash == *plutus_v1_script_hash {
                *covered = true;
                *plutus_v1_script_covered = true;
            }
        }
        for (plutus_v2_script_covered, plutus_v2_script_hash) in plutus_v2_scripts.iter_mut() {
            if *hash == *plutus_v2_script_hash {
                *covered = true;
                *plutus_v2_script_covered = true;
            }
        }
        for (plutus_v3_script_covered, plutus_v3_script_hash) in pluts_v3_scripts.iter_mut() {
            if *hash == *plutus_v3_script_hash {
                *covered = true;
                *plutus_v3_script_covered = true;
            }
        }
    }
    for (covered, hash) in needed_input_scripts {
        if !covered && !reference_scripts.contains(&hash) {
            return Err(PostAlonzo(ScriptWitnessMissing));
        }
    }
    Ok(())
}

fn get_script_hashes_from_inputs(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
) -> Vec<(bool, ScriptHash)> {
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
        .and_then(MultiEraOutput::as_conway)
    {
        Some(TransactionOutput::Legacy(output)) => match get_payment_part(&output.address) {
            Some(ShelleyPaymentPart::Script(script_hash)) => Some(script_hash),
            _ => None,
        },
        Some(TransactionOutput::PostAlonzo(output)) => match get_payment_part(&output.address) {
            Some(ShelleyPaymentPart::Script(script_hash)) => Some(script_hash),
            _ => None,
        },
        None => None,
    }
}

fn get_script_hash_from_reference_input(
    ref_input: &TransactionInput,
    utxos: &UTxOs,
) -> Option<PolicyId> {
    match utxos
        .get(&MultiEraInput::from_alonzo_compatible(ref_input))
        .and_then(MultiEraOutput::as_conway)
    {
        Some(TransactionOutput::Legacy(_)) => None,
        Some(TransactionOutput::PostAlonzo(output)) => {
            if let Some(script_ref_cborwrap) = &output.script_ref {
                match script_ref_cborwrap.clone().unwrap() {
                    ScriptRef::NativeScript(native_script) => {
                        // First, the NativeScript header.
                        let mut val_to_hash: Vec<u8> = vec![0];
                        // Then, the CBOR content.
                        val_to_hash.extend_from_slice(native_script.raw_cbor());
                        return Some(pallas_crypto::hash::Hasher::<224>::hash(&val_to_hash));
                    }
                    ScriptRef::PlutusV1Script(plutus_v1_script) => {
                        // First, the PlutusV1Script header.
                        let mut val_to_hash: Vec<u8> = vec![1];
                        // Then, the CBOR content.
                        val_to_hash.extend_from_slice(plutus_v1_script.as_ref());
                        return Some(pallas_crypto::hash::Hasher::<224>::hash(&val_to_hash));
                    }
                    ScriptRef::PlutusV2Script(plutus_v2_script) => {
                        // First, the PlutusV2Script header.
                        let mut val_to_hash: Vec<u8> = vec![2];
                        // Then, the CBOR content.
                        val_to_hash.extend_from_slice(plutus_v2_script.as_ref());
                        return Some(pallas_crypto::hash::Hasher::<224>::hash(&val_to_hash));
                    }
                    ScriptRef::PlutusV3Script(plutus_v3_script) => {
                        // First, the PlutusV2Script header.
                        let mut val_to_hash: Vec<u8> = vec![3];
                        // Then, the CBOR content.
                        val_to_hash.extend_from_slice(plutus_v3_script.as_ref());
                        return Some(pallas_crypto::hash::Hasher::<224>::hash(&val_to_hash));
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn check_minting_policies(
    tx_body: &TransactionBody,
    native_scripts: &mut [(bool, PolicyId)],
    plutus_v1_scripts: &mut [(bool, PolicyId)],
    plutus_v2_scripts: &mut [(bool, PolicyId)],
    plutus_v3_scripts: &mut [(bool, PolicyId)],
    reference_scripts: &[PolicyId],
) -> ValidationResult {
    match &tx_body.mint {
        None => Ok(()),
        Some(minted_value) => {
            let mut minting_policies: Vec<(bool, PolicyId)> =
                minted_value.keys().map(|pol| (false, *pol)).collect();
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
                for (plutus_script_covered, plutus_v3_script_hash) in plutus_v3_scripts.iter_mut() {
                    if *policy == *plutus_v3_script_hash {
                        *policy_covered = true;
                        *plutus_script_covered = true;
                    }
                }
                for reference_script_hash in reference_scripts.iter() {
                    if *policy == *reference_script_hash {
                        *policy_covered = true;
                    }
                }
            }
            for (policy_covered, policy) in minting_policies {
                if !policy_covered {
                    return Err(PostAlonzo(MintingLacksPolicy(policy)));
                }
            }
            Ok(())
        }
    }
}

// Each datum hash in a Plutus script input matches the hash of a datum in the
// transaction witness set
fn check_datums(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    option_plutus_data: &Option<Vec<KeepRaw<PlutusData>>>,
) -> ValidationResult {
    let mut plutus_data_hashes: Vec<(bool, Hash<32>)> = match option_plutus_data {
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
    check_input_datum_hash_in_witness_set(tx_body, utxos, &mut plutus_data_hashes)?;
    check_remaining_datums(&plutus_data_hashes, tx_body, utxos)
}

// Each datum hash in a Plutus script input matches the hash of a datum in the
// transaction witness set.
fn check_input_datum_hash_in_witness_set(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    plutus_data_hash: &mut [(bool, Hash<32>)],
) -> ValidationResult {
    for input in &tx_body.inputs {
        let output = utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .ok_or(PostAlonzo(InputNotInUTxO))?;

        // we only check for datum in the witness set if it's not an inline datum in the
        // output (aka: DatumOption::Hash).
        if let Some(DatumOption::Hash(hash)) = output.datum() {
            find_plutus_datum_in_witness_set(&hash, plutus_data_hash)?
        }
    }

    Ok(())
}

// Extract datum hash if one is contained.
fn get_datum_hash(output: &TransactionOutput) -> Option<Hash<32>> {
    match output {
        TransactionOutput::Legacy(output) => output.datum_hash,
        TransactionOutput::PostAlonzo(output) => match output.datum_option.as_deref() {
            Some(DatumOption::Hash(hash)) => Some(*hash),
            _ => None,
        },
    }
}

fn find_plutus_datum_in_witness_set(
    hash: &Hash<32>,
    plutus_data_hash: &mut [(bool, Hash<32>)],
) -> ValidationResult {
    for (found, plutus_datum_hash) in plutus_data_hash {
        if hash == plutus_datum_hash {
            *found = true;
            return Ok(());
        }
    }
    Err(PostAlonzo(DatumMissing))
}

// Each datum in the transaction witness set can be related to the datum hash in
// a Plutus script input, or in a reference input, or in a regular output, or in
// the collateral return output
fn check_remaining_datums(
    plutus_data_hash: &[(bool, Hash<32>)],
    tx_body: &TransactionBody,
    utxos: &UTxOs,
) -> ValidationResult {
    for (found, plutus_datum_hash) in plutus_data_hash {
        if !found {
            find_datum(plutus_datum_hash, tx_body, utxos)?
        }
    }
    Ok(())
}

fn find_datum(hash: &Hash<32>, tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
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
            TransactionOutput::Legacy(output) => {
                if let Some(datum_hash) = &output.datum_hash {
                    if *hash == *datum_hash {
                        return Ok(());
                    }
                }
            }
            TransactionOutput::PostAlonzo(output) => {
                if let Some(DatumOption::Hash(datum_hash)) = &output.datum_option.as_deref() {
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
                .and_then(MultiEraOutput::as_conway)
            {
                Some(TransactionOutput::Legacy(output)) => {
                    if let Some(datum_hash) = &output.datum_hash {
                        if *hash == *datum_hash {
                            return Ok(());
                        }
                    }
                }
                Some(TransactionOutput::PostAlonzo(output)) => {
                    if let Some(DatumOption::Hash(datum_hash)) = &output.datum_option.as_deref() {
                        if *hash == *datum_hash {
                            return Ok(());
                        }
                    }
                }
                _ => (),
            }
        }
    }
    Err(PostAlonzo(UnneededDatum))
}

fn check_redeemers(
    plutus_v1_scripts: &[PolicyId],
    plutus_v2_scripts: &[PolicyId],
    plutus_v3_scripts: &[PolicyId],
    reference_scripts: &[PolicyId],
    tx_body: &TransactionBody,
    tx_wits: &WitnessSet,
    utxos: &UTxOs,
) -> ValidationResult {
    let redeemer_key: Vec<RedeemersKey> = match &tx_wits.redeemer {
        Some(redeemers) => match redeemers.clone().unwrap() {
            Redeemers::List(redeemers) => redeemers
                .iter()
                .map(|x| RedeemersKey {
                    tag: x.tag,
                    index: x.index,
                })
                .collect(),
            Redeemers::Map(redeemers) => redeemers
                .iter()
                .map(|x| RedeemersKey {
                    tag: x.0.tag,
                    index: x.0.index,
                })
                .collect(),
        },

        None => Vec::new(),
    };

    let plutus_scripts: Vec<RedeemersKey> = mk_plutus_script_redeemer_pointers(
        plutus_v1_scripts,
        plutus_v2_scripts,
        plutus_v3_scripts,
        reference_scripts,
        tx_body,
        utxos,
    );
    redeemer_key_coincide(&redeemer_key, &plutus_scripts)
}

// Lexicographical sorting for inputs.
fn sort_inputs(unsorted_inputs: &[TransactionInput]) -> Vec<TransactionInput> {
    let mut res: Vec<TransactionInput> = unsorted_inputs.to_owned();
    res.sort();
    res
}

fn mk_plutus_script_redeemer_pointers(
    plutus_v1_scripts: &[PolicyId],
    plutus_v2_scripts: &[PolicyId],
    plutus_v3_scripts: &[PolicyId],
    reference_scripts: &[PolicyId],
    tx_body: &TransactionBody,
    utxos: &UTxOs,
) -> Vec<RedeemersKey> {
    let mut res: Vec<RedeemersKey> = Vec::new();
    let sorted_inputs: &Vec<TransactionInput> = &sort_inputs(&tx_body.inputs);
    for (index, input) in sorted_inputs.iter().enumerate() {
        if let Some(script_hash) = get_script_hash_from_input(input, utxos) {
            // Only create redeemer pointer for Plutus scripts, not native scripts
            if is_phase_2_script(
                &script_hash,
                plutus_v1_scripts,
                plutus_v2_scripts,
                plutus_v3_scripts,
                reference_scripts,
            ) {
                res.push(RedeemersKey {
                    tag: pallas_primitives::conway::RedeemerTag::Spend,
                    index: index as u32,
                })
            }
        }
    }
    if let Some(mint) = &tx_body.mint {
        for (index, policy) in sort_policies(mint).iter().enumerate() {
            if is_phase_2_script(
                policy,
                plutus_v1_scripts,
                plutus_v2_scripts,
                plutus_v3_scripts,
                reference_scripts,
            ) {
                res.push(RedeemersKey {
                    tag: pallas_primitives::conway::RedeemerTag::Mint,
                    index: index as u32,
                })
            }
        }
    }
    if let Some(withdrawals) = &tx_body.withdrawals {
        for (index, (stake_key_hash_bytes, _amount)) in withdrawals.iter().enumerate() {
            let addr = Address::from_bytes(stake_key_hash_bytes)
                .ok_or(PostAlonzo(InputDecoding))?;
            if let Address::Stake(stake_addr) = addr {
                if stake_addr.is_script() {
                    let script_hash = stake_addr.payload().as_hash();
                    if is_phase_2_script(
                        &script_hash,
                        plutus_v1_scripts,
                        plutus_v2_scripts,
                        plutus_v3_scripts,
                        reference_scripts,
                    ) {
                        res.push(RedeemersKey {
                            tag: pallas_primitives::conway::RedeemerTag::Reward,
                            index: index as u32,
                        })
                    }
                }
            }
        }
    }
    res
}

// Lexicographical sorting for PolicyID's.
fn sort_policies(mint: &Mint) -> Vec<PolicyId> {
    let mut res: Vec<PolicyId> = mint.clone().keys().copied().collect();
    res.sort();
    res
}

fn is_phase_2_script(
    policy: &PolicyId,
    plutus_v1_scripts: &[PolicyId],
    plutus_v2_scripts: &[PolicyId],
    plutus_v3_scripts: &[PolicyId],
    reference_scripts: &[PolicyId],
) -> bool {
    plutus_v1_scripts
        .iter()
        .any(|v1_script| policy == v1_script)
        || plutus_v2_scripts
            .iter()
            .any(|v2_script| policy == v2_script)
        || plutus_v3_scripts
            .iter()
            .any(|v3_script| policy == v3_script)
        || reference_scripts
            .iter()
            .any(|ref_script| policy == ref_script)
}

fn redeemer_key_coincide(
    redeemers: &[RedeemersKey],
    plutus_scripts: &[RedeemersKey],
) -> ValidationResult {
    for redeemer_pointer in redeemers {
        if !plutus_scripts.iter().any(|x| x == redeemer_pointer) {
            return Err(PostAlonzo(UnneededRedeemer));
        }
    }
    for ps_redeemer_pointer in plutus_scripts {
        if !redeemers.iter().any(|x| x == ps_redeemer_pointer) {
            return Err(PostAlonzo(RedeemerMissing));
        }
    }
    Ok(())
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
            None => return Err(PostAlonzo(ReqSignerMissing)),
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
                return Err(PostAlonzo(ReqSignerWrongSig));
            } else {
                return Ok(());
            }
        }
    }
    Err(PostAlonzo(ReqSignerMissing))
}

fn check_vkey_input_wits(
    mtx: &Tx,
    vkey_wits: &Option<Vec<VKeyWitness>>,
    utxos: &UTxOs,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body;
    let vk_wits: &mut Vec<(bool, VKeyWitness)> =
        &mut mk_alonzo_vk_wits_check_list(vkey_wits, PostAlonzo(VKWitnessMissing))?;
    let tx_hash: &Vec<u8> = &Vec::from(mtx.transaction_body.original_hash().as_ref());
    let mut inputs_and_collaterals: Vec<TransactionInput> = Vec::new();
    inputs_and_collaterals.extend(tx_body.inputs.clone().to_vec());
    if let Some(collaterals) = &tx_body.collateral {
        inputs_and_collaterals.extend(collaterals.clone().to_vec())
    }
    for input in inputs_and_collaterals.iter() {
        match utxos.get(&MultiEraInput::from_alonzo_compatible(input)) {
            Some(multi_era_output) => {
                if let Some(babbage_output) = MultiEraOutput::as_conway(multi_era_output) {
                    let address: &Bytes = match babbage_output {
                        TransactionOutput::Legacy(output) => &output.address,
                        TransactionOutput::PostAlonzo(output) => &output.address,
                    };
                    match get_payment_part(address).ok_or(PostAlonzo(InputDecoding))? {
                        ShelleyPaymentPart::Key(payment_key_hash) => {
                            check_vk_wit(&payment_key_hash, vk_wits, tx_hash)?
                        }
                        ShelleyPaymentPart::Script(_) => (),
                    }
                }
            }
            None => return Err(PostAlonzo(InputNotInUTxO)),
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
                return Err(PostAlonzo(VKWrongSignature));
            } else {
                *vkey_wit_covered = true;
                return Ok(());
            }
        }
    }
    Err(PostAlonzo(VKWitnessMissing))
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
                return Err(PostAlonzo(VKWrongSignature));
            }
        }
    }
    Ok(())
}

fn check_languages(mtx: &Tx, utxos: &UTxOs, prot_pps: &ConwayProtParams) -> ValidationResult {
    let used_langs = tx_languages(mtx, utxos);
    let allowed_langs: Vec<Language> = allowed_tx_langs(mtx, utxos);
    let available_langs: Vec<Language> = available_langs(prot_pps);

    for tx_lang in used_langs.iter() {
        if !available_langs.contains(tx_lang) && !allowed_langs.contains(tx_lang) {
            return Err(PostAlonzo(UnsupportedPlutusLanguage));
        }
    }

    Ok(())
}

fn available_langs(prot_pps: &ConwayProtParams) -> Vec<Language> {
    let mut res: Vec<Language> = Vec::new();

    if prot_pps
        .cost_models_for_script_languages
        .plutus_v1
        .is_some()
    {
        res.push(Language::PlutusV1);
    }

    if prot_pps
        .cost_models_for_script_languages
        .plutus_v2
        .is_some()
    {
        res.push(Language::PlutusV2);
    }

    if prot_pps
        .cost_models_for_script_languages
        .plutus_v3
        .is_some()
    {
        res.push(Language::PlutusV3);
    }

    res
}

fn allowed_tx_langs(mtx: &Tx, utxos: &UTxOs) -> Vec<Language> {
    let all_outputs: Vec<&TransactionOutput> = compute_all_outputs(mtx, utxos);
    if any_byron_addresses(&all_outputs) {
        vec![]
    } else if any_datums_or_script_refs(&all_outputs)
        || any_reference_inputs(
            &mtx.transaction_body
                .reference_inputs
                .clone()
                .map(|x| x.to_vec()),
        )
    {
        vec![Language::PlutusV2, Language::PlutusV3]
    } else {
        vec![Language::PlutusV1, Language::PlutusV2, Language::PlutusV3]
    }
}

fn compute_all_outputs<'a>(mtx: &'a Tx, utxos: &'a UTxOs) -> Vec<&'a TransactionOutput<'a>> {
    let mut res: Vec<&TransactionOutput> = Vec::new();
    for input in mtx.transaction_body.inputs.iter() {
        if let Some(output) = utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .and_then(MultiEraOutput::as_conway)
        {
            res.push(output)
        }
    }
    if let Some(reference_inputs) = &mtx.transaction_body.reference_inputs {
        for ref_input in reference_inputs.iter() {
            if let Some(output) = utxos
                .get(&MultiEraInput::from_alonzo_compatible(ref_input))
                .and_then(MultiEraOutput::as_conway)
            {
                res.push(output)
            }
        }
    }
    for output in mtx.transaction_body.outputs.iter() {
        res.push(output)
    }
    res
}

fn any_byron_addresses(all_outputs: &[&TransactionOutput]) -> bool {
    for output in all_outputs.iter() {
        match output {
            TransactionOutput::Legacy(output) => {
                if is_byron_address(&output.address) {
                    return true;
                }
            }
            TransactionOutput::PostAlonzo(output) => {
                if is_byron_address(&output.address) {
                    return true;
                }
            }
        }
    }
    false
}

fn any_datums_or_script_refs(all_outputs: &[&TransactionOutput]) -> bool {
    for output in all_outputs.iter() {
        match output {
            TransactionOutput::Legacy(_) => (),
            TransactionOutput::PostAlonzo(output) => {
                if output.script_ref.is_some() {
                    return true;
                } else if let Some(DatumOption::Data(_)) = &output.datum_option.as_deref() {
                    return true;
                }
            }
        }
    }
    false
}

fn any_reference_inputs(reference_inputs: &Option<Vec<TransactionInput>>) -> bool {
    match reference_inputs {
        Some(reference_inputs) => !reference_inputs.is_empty(),
        None => false,
    }
}

fn tx_languages(mtx: &Tx, utxos: &UTxOs) -> Vec<Language> {
    let mut v1_scripts: bool = false;
    let mut v2_scripts: bool = false;
    let mut v3_scripts: bool = false;
    if let Some(v1_scripts_vec) = &mtx.transaction_witness_set.plutus_v1_script {
        if !v1_scripts_vec.is_empty() {
            v1_scripts = true
        }
    }
    if let Some(v2_scripts_vec) = &mtx.transaction_witness_set.plutus_v2_script {
        if !v2_scripts_vec.is_empty() {
            v2_scripts = true;
        }
    }
    if let Some(v3_scripts_vec) = &mtx.transaction_witness_set.plutus_v3_script {
        if !v3_scripts_vec.is_empty() {
            v3_scripts = true;
        }
    }
    if let Some(reference_inputs) = &mtx.transaction_body.reference_inputs {
        for ref_input in reference_inputs.iter() {
            if let Some(TransactionOutput::PostAlonzo(output)) = utxos
                .get(&MultiEraInput::from_alonzo_compatible(ref_input))
                .and_then(MultiEraOutput::as_conway)
            {
                if let Some(script_ref_cborwrap) = &output.script_ref {
                    match script_ref_cborwrap.clone().unwrap() {
                        ScriptRef::PlutusV1Script(_) => v1_scripts = true,
                        ScriptRef::PlutusV2Script(_) => v2_scripts = true,
                        ScriptRef::PlutusV3Script(_) => v3_scripts = true,
                        _ => (),
                    }
                }
            }
        }
    }
    if !v1_scripts && !v2_scripts && !v3_scripts {
        vec![]
    } else if v1_scripts && !v2_scripts && !v3_scripts {
        vec![Language::PlutusV1]
    } else if !v1_scripts && v2_scripts && !v3_scripts {
        vec![Language::PlutusV2]
    } else if !v1_scripts && !v2_scripts && v3_scripts {
        vec![Language::PlutusV3]
    } else {
        vec![Language::PlutusV1, Language::PlutusV2]
    }
}

// The metadata of the transaction is valid.
fn check_auxiliary_data(tx_body: &TransactionBody, mtx: &Tx) -> ValidationResult {
    match (&tx_body.auxiliary_data_hash, aux_data_from_conway_tx(mtx)) {
        (Some(metadata_hash), Some(metadata)) => {
            if metadata_hash.as_slice()
                == pallas_crypto::hash::Hasher::<256>::hash(metadata).as_ref()
            {
                Ok(())
            } else {
                Err(PostAlonzo(MetadataHash))
            }
        }
        (None, None) => Ok(()),
        _ => Err(PostAlonzo(MetadataHash)),
    }
}

fn check_script_data_hash(
    tx_body: &TransactionBody,
    mtx: &Tx,
    utxos: &UTxOs,
    prot_pps: &ConwayProtParams,
) -> ValidationResult {
    let tx_languages = tx_languages(mtx, utxos);

    let Some(provided) = tx_body.script_data_hash else {
        if tx_languages.is_empty() {
            return Ok(());
        } else {
            return Err(PostAlonzo(ScriptIntegrityHash));
        }
    };

    let Some(language_view) = cost_model_for_tx(&tx_languages, prot_pps) else {
        return Err(PostAlonzo(ScriptIntegrityHash));
    };

    let expected = pallas_primitives::conway::ScriptData::build_for(
        &mtx.transaction_witness_set,
        language_view,
    )
    .ok_or(PostAlonzo(ScriptIntegrityHash))?
    .hash();

    if provided != expected {
        return Err(PostAlonzo(ScriptIntegrityHash));
    }

    Ok(())
}

fn cost_model_for_tx(
    tx_languages: &[Language],
    prot_pps: &ConwayProtParams,
) -> Option<LanguageView> {
    let lang = itertools::max(tx_languages.iter())?;

    let costs = match lang {
        Language::PlutusV1 => prot_pps.cost_models_for_script_languages.plutus_v1.clone(),
        Language::PlutusV2 => prot_pps.cost_models_for_script_languages.plutus_v2.clone(),
        Language::PlutusV3 => prot_pps.cost_models_for_script_languages.plutus_v3.clone(),
    };

    costs.map(|costs| LanguageView(lang.clone() as u8, costs))
}
