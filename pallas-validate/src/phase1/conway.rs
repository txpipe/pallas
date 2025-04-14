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
use pallas_addresses::{ScriptHash, ShelleyAddress, ShelleyPaymentPart};
use pallas_codec::{
    minicbor::{encode, Encoder},
    utils::{Bytes, KeepRaw},
};
use pallas_primitives::{
    babbage,
    conway::{
        DatumOption, Language, Mint, NativeScript, Redeemers, RedeemersKey, RequiredSigners,
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
    network_magic: &u32,
    network_id: &u8,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body.clone();
    let size: u32 = get_conway_tx_size(mtx).ok_or(PostAlonzo(UnknownTxSize))?;
    check_data_business_invariants(mtx, tx_body)?;
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
    check_minting(tx_body, mtx)?;
    check_well_formedness(tx_body, mtx)?;
    check_witness_set(mtx, utxos)?;
    check_languages(mtx, utxos, network_magic, network_id, block_slot)?;
    check_auxiliary_data(tx_body, mtx)?;
    check_script_data_hash(tx_body, mtx, utxos, network_magic, network_id, block_slot)
}

fn check_data_business_invariants(mtx: &Tx, tx_body: &TransactionBody) -> ValidationResult {
    let witness_set: &WitnessSet = &mtx.transaction_witness_set;
    let body_invariants: bool =
        tx_body.certificates.as_ref().is_none_or(|x| x.validate_data()) &&
        tx_body.collateral.as_ref().is_none_or(|x| x.validate_data()) &&
        tx_body.required_signers.as_ref().is_none_or(|x| x.validate_data()) &&
        tx_body.reference_inputs.as_ref().is_none_or(|x| x.validate_data()) &&
        tx_body.proposal_procedures.as_ref().is_none_or(|x| x.validate_data());
    let witness_invariants: bool =
        witness_set.vkeywitness.as_ref().is_none_or(|x| x.validate_data()) &&
        witness_set.native_script.as_ref().is_none_or(|x| x.validate_data()) &&
        witness_set.bootstrap_witness.as_ref().is_none_or(|x| x.validate_data()) &&
        witness_set.plutus_v1_script.as_ref().is_none_or(|x| x.validate_data()) &&
        witness_set.plutus_data.as_ref().is_none_or(|x| x.validate_data()) &&
        witness_set.plutus_v2_script.as_ref().is_none_or(|x| x.validate_data()) &&
        witness_set.plutus_v3_script.as_ref().is_none_or(|x| x.validate_data());
    if body_invariants && witness_invariants {
        Ok(())
    } else {
        Err(PostAlonzo(BrokenBusinessInvariant))
    }
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
    match multi_era_output {
        MultiEraOutput::Conway(x) => match x.clone().into_owned() {
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
        },
        MultiEraOutput::AlonzoCompatible(output, _) => {
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
        _ => unimplemented!(),
    }
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
fn check_minting(tx_body: &TransactionBody, mtx: &Tx) -> ValidationResult {
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
            let v1_script_wits: Vec<PlutusScript<1>> =
                match &mtx.transaction_witness_set.plutus_v1_script {
                    None => Vec::new(),
                    Some(v1_script_wits) => v1_script_wits.clone().to_vec(),
                };
            let v2_script_wits: Vec<PlutusScript<2>> =
                match &mtx.transaction_witness_set.plutus_v2_script {
                    None => Vec::new(),
                    Some(v2_script_wits) => v2_script_wits.clone().to_vec(),
                };
            let v3_script_wits: Vec<PlutusScript<3>> =
                match &mtx.transaction_witness_set.plutus_v3_script {
                    None => Vec::new(),
                    Some(v3_script_wits) => v3_script_wits.clone().to_vec(),
                };
            for (policy, _) in minted_value.iter() {
                if native_script_wits
                    .iter()
                    .all(|script| compute_native_script_hash(script) != *policy)
                    && v1_script_wits
                        .iter()
                        .all(|script| compute_plutus_v1_script_hash(script) != *policy)
                    && v2_script_wits
                        .iter()
                        .all(|script| compute_plutus_v2_script_hash(script) != *policy)
                    && v3_script_wits
                        .iter()
                        .all(|script| compute_plutus_v3_script_hash(script) != *policy)
                {
                    return Err(PostAlonzo(MintingLacksPolicy));
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
    filtered_native_scripts.retain(|&(_, native_script_hash)| {
        !reference_scripts
            .iter()
            .any(|&reference_script_hash| reference_script_hash == native_script_hash)
    });
    let mut filtered_plutus_v1_scripts: Vec<(bool, PolicyId)> = plutus_v1_scripts
        .iter()
        .map(|&script_hash| (false, script_hash))
        .collect();
    filtered_plutus_v1_scripts.retain(|&(_, plutus_v1_script_hash)| {
        !reference_scripts
            .iter()
            .any(|&reference_script_hash| reference_script_hash == plutus_v1_script_hash)
    });
    let mut filtered_plutus_v2_scripts: Vec<(bool, PolicyId)> = plutus_v2_scripts
        .iter()
        .map(|&script_hash| (false, script_hash))
        .collect();
    filtered_plutus_v2_scripts.retain(|&(_, plutus_v2_script_hash)| {
        !reference_scripts
            .iter()
            .any(|&reference_script_hash| reference_script_hash == plutus_v2_script_hash)
    });
    let mut filtered_plutus_v3_scripts: Vec<(bool, PolicyId)> = plutus_v3_scripts
        .iter()
        .map(|&script_hash| (false, script_hash))
        .collect();
    filtered_plutus_v3_scripts.retain(|&(_, plutus_v3_script_hash)| {
        !reference_scripts
            .iter()
            .any(|&reference_script_hash| reference_script_hash == plutus_v3_script_hash)
    });
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
        if !covered
            && !reference_scripts
                .iter()
                .any(|reference_script_hash| *reference_script_hash == hash)
        {
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
            for (policy_covered, _) in minting_policies {
                if !policy_covered {
                    return Err(PostAlonzo(MintingLacksPolicy));
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
        match utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .and_then(MultiEraOutput::as_conway)
        {
            Some(output) => {
                if let Some(datum_hash) = get_datum_hash(output) {
                    find_plutus_datum_in_witness_set(&datum_hash, plutus_data_hash)?
                }
            }
            None => return Err(PostAlonzo(InputNotInUTxO)),
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
        if get_script_hash_from_input(input, utxos).is_some() {
            res.push(RedeemersKey {
                tag: pallas_primitives::conway::RedeemerTag::Spend,
                index: index as u32,
            })
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

fn check_languages(
    mtx: &Tx,
    utxos: &UTxOs,
    network_magic: &u32,
    network_id: &u8,
    block_slot: &u64,
) -> ValidationResult {
    let available_langs: Vec<Language> =
        available_langs(mtx, utxos, network_magic, network_id, block_slot);
    for tx_lang in tx_languages(mtx, utxos).iter() {
        if !available_langs
            .iter()
            .any(|available_lang| *available_lang == *tx_lang)
        {
            return Err(PostAlonzo(UnsupportedPlutusLanguage));
        }
    }
    Ok(())
}

fn available_langs(
    mtx: &Tx,
    utxos: &UTxOs,
    network_magic: &u32,
    network_id: &u8,
    block_slot: &u64,
) -> Vec<Language> {
    let block_langs: Vec<Language> = block_langs(*network_magic, *network_id, *block_slot);
    let allowed_langs: Vec<Language> = allowed_langs(mtx, utxos);
    block_langs
        .iter()
        .filter(|&cost_model_language| allowed_langs.contains(cost_model_language))
        .cloned()
        .collect::<Vec<Language>>()
}

fn block_langs(network_magic: u32, network_id: u8, block_slot: u64) -> Vec<Language> {
    if network_magic == 1 && network_id == 0 {
        //Preprod - 3,974,409 is the slot of the first block in epoch 13
        if block_slot >= 3974409 {
            vec![Language::PlutusV1, Language::PlutusV2, Language::PlutusV3]
        } else {
            vec![Language::PlutusV1]
        }
    } else if network_magic == 2 && network_id == 0 {
        // Preview - 777,610 is the slot of the first block in epoch 9
        if block_slot >= 777610 {
            vec![Language::PlutusV1, Language::PlutusV2, Language::PlutusV3]
        } else {
            vec![Language::PlutusV1]
        }
    } else {
        // Mainnet - 72,748,820 is the slot of the first block in epoch 366
        if block_slot >= 72748820 {
            vec![Language::PlutusV1, Language::PlutusV2, Language::PlutusV3]
        } else {
            vec![Language::PlutusV1]
        }
    }
}

fn allowed_langs(mtx: &Tx, utxos: &UTxOs) -> Vec<Language> {
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
        vec![Language::PlutusV1, Language::PlutusV2]
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
    network_magic: &u32,
    network_id: &u8,
    block_slot: &u64,
) -> ValidationResult {
    match tx_body.script_data_hash {
        Some(script_data_hash) => match (
            &mtx.transaction_witness_set.plutus_data,
            &mtx.transaction_witness_set.redeemer,
        ) {
            (Some(plutus_data), Some(redeemer)) => {
                let plutus_data: Vec<PlutusData> = plutus_data
                    .iter()
                    .map(|x| KeepRaw::unwrap(x.clone()))
                    .collect();
                // The Plutus data part of the script integrity hash may either need to be
                // serialized as a indefinite-length array, or a definite-length one.
                // TODO: compute only the correct hash, not both of them.
                let (indefinite_hash, definite_hash) = compute_script_integrity_hash(
                    &tx_languages(mtx, utxos),
                    &plutus_data,
                    &redeemer.clone().unwrap(),
                    network_magic,
                    network_id,
                    block_slot,
                );

                if script_data_hash == indefinite_hash || script_data_hash == definite_hash {
                    Ok(())
                } else {
                    Err(PostAlonzo(ScriptIntegrityHash))
                }
            }
            (None, Some(redeemer)) => {
                let plutus_data: Vec<PlutusData> = vec![];
                // The Plutus data part of the script integrity hash may either need to be
                // serialized as a indefinite-length array, or a definite-length one.
                // TODO: compute only the correct hash, not both of them.
                let (indefinite_hash, definite_hash) = compute_script_integrity_hash(
                    &tx_languages(mtx, utxos),
                    &plutus_data,
                    &redeemer.clone().unwrap(),
                    network_magic,
                    network_id,
                    block_slot,
                );

                if script_data_hash == indefinite_hash || script_data_hash == definite_hash {
                    Ok(())
                } else {
                    Err(PostAlonzo(ScriptIntegrityHash))
                }
            }
            (None, None) => Err(PostAlonzo(ScriptIntegrityHash)),
            (Some(_), None) => Err(PostAlonzo(ScriptIntegrityHash)),
        },
        None => {
            let plutus_data = mtx
                .transaction_witness_set
                .plutus_data
                .clone()
                .map(|x| x.to_vec());
            let redeemer_is_empty = mtx.transaction_witness_set.redeemer.is_none();

            if !redeemer_is_empty {
                if option_vec_is_empty(&plutus_data) {
                    match &mtx
                        .transaction_witness_set
                        .redeemer
                        .clone()
                        .unwrap()
                        .unwrap()
                    {
                        Redeemers::List(redeemers) => {
                            if !option_vec_is_empty(&Some(redeemers.clone().to_vec())) {
                                return Err(PostAlonzo(ScriptIntegrityHash));
                            }
                        }
                        Redeemers::Map(redeemers) => {
                            if !option_vec_is_empty(&Some(redeemers.clone().iter().collect())) {
                                return Err(PostAlonzo(ScriptIntegrityHash));
                            }
                        }
                    }
                    Ok(())
                } else {
                    Err(PostAlonzo(ScriptIntegrityHash))
                }
            } else {
                Ok(())
            }
        }
    }
}

// The Plutus data is encoded both as an indefinite-length and a definite-length
// array. Hence, two hashes are computed.
// TODO: compute only the necessary form, which requires knowing the original
// encoding in the MintedWitnessSet.
fn compute_script_integrity_hash(
    tx_languages: &[Language],
    plutus_data: &[PlutusData],
    redeemer: &Redeemers,
    network_magic: &u32,
    network_id: &u8,
    block_slot: &u64,
) -> (Hash<32>, Hash<32>) {
    // Indefinite Plutus data serialization
    let mut value_to_hash_with_indef: Vec<u8> = Vec::new();
    let _ = encode(redeemer, &mut value_to_hash_with_indef);
    if !plutus_data.is_empty() {
        let mut plutus_data_encoder_indef: Encoder<Vec<u8>> = Encoder::new(Vec::new());
        let _ = plutus_data_encoder_indef.begin_array();
        for single_plutus_data in plutus_data.iter() {
            let _ = plutus_data_encoder_indef.encode(single_plutus_data);
        }
        let _ = plutus_data_encoder_indef.end();
        value_to_hash_with_indef.extend(plutus_data_encoder_indef.writer().clone());
    }
    let cost_model = cost_model_cbor(tx_languages, *network_magic, *network_id, block_slot);
    value_to_hash_with_indef.extend(&cost_model);
    // Definite Plutus data serialization
    let mut value_to_hash_with_def: Vec<u8> = Vec::new();
    let _ = encode(redeemer, &mut value_to_hash_with_def);
    if !plutus_data.is_empty() {
        let mut plutus_data_encoder_def: Encoder<Vec<u8>> = Encoder::new(Vec::new());
        let _ = plutus_data_encoder_def.array(plutus_data.len() as u64);
        for single_plutus_data in plutus_data.iter() {
            let _ = plutus_data_encoder_def.encode(single_plutus_data);
        }
        value_to_hash_with_def.extend(plutus_data_encoder_def.writer().clone());
    }
    value_to_hash_with_def.extend(&cost_model);
    (
        pallas_crypto::hash::Hasher::<256>::hash(&value_to_hash_with_indef),
        pallas_crypto::hash::Hasher::<256>::hash(&value_to_hash_with_def),
    )
}

// Precondition: !tx_languages.is_empty()
fn cost_model_cbor(
    tx_languages: &[Language],
    network_magic: u32,
    network_id: u8,
    block_slot: &u64,
) -> Vec<u8> {
    if network_magic == 1 && network_id == 0 {
        // Preprod
        if *block_slot < 3974409 {
            // Up to end of epoch 12
            hex::decode(
                "a141005901d59f1a000302590001011a00060bc719026d00011a000249f01903e800011a000249f018201a0025cea81971f70419744d186419744d186419744d186419744d186419744d186419744d18641864186419744d18641a000249f018201a000249f018201a000249f018201a000249f01903e800011a000249f018201a000249f01903e800081a000242201a00067e2318760001011a000249f01903e800081a000249f01a0001b79818f7011a000249f0192710011a0002155e19052e011903e81a000249f01903e8011a000249f018201a000249f018201a000249f0182001011a000249f0011a000249f0041a000194af18f8011a000194af18f8011a0002377c190556011a0002bdea1901f1011a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000242201a00067e23187600010119f04c192bd200011a000249f018201a000242201a00067e2318760001011a000242201a00067e2318760001011a0025cea81971f704001a000141bb041a000249f019138800011a000249f018201a000302590001011a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a00330da70101ff"
            ).unwrap()
        } else if (3974409..=20390403).contains(block_slot) {
            // From start of epoch 13 up to end of epoch 50
            if tx_languages.contains(&Language::PlutusV1)
                && !tx_languages.contains(&Language::PlutusV2)
            {
                hex::decode(
                    "a141005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a009063b91903fd0aff"
                ).unwrap()
            } else if !tx_languages.contains(&Language::PlutusV1)
                && tx_languages.contains(&Language::PlutusV2)
            {
                hex::decode(
                    "a10198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201b00000004a817c8001b00000004a817c8001a009063b91903fd0a1b00000004a817c800001b00000004a817c800"
                ).unwrap()
            } else {
                hex::decode(
                    "a241005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a009063b91903fd0aff0198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201b00000004a817c8001b00000004a817c8001a009063b91903fd0a1b00000004a817c800001b00000004a817c800"
                ).unwrap()
            }
        } else {
            // From start of epoch 51 onwards
            if tx_languages.contains(&Language::PlutusV1)
                && !tx_languages.contains(&Language::PlutusV2)
                && !tx_languages.contains(&Language::PlutusV3)
            {
                hex::decode(
                    "a141005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0374f693194a1f0aff"
                ).unwrap()
            } else if !tx_languages.contains(&Language::PlutusV1)
                && tx_languages.contains(&Language::PlutusV2)
                && !tx_languages.contains(&Language::PlutusV3)
            {
                hex::decode(
                    "a10198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0223accc0a1a0374f693194a1f0a1a02515e841980b30a"
                ).unwrap()
            } else if !tx_languages.contains(&Language::PlutusV1)
                && !tx_languages.contains(&Language::PlutusV2)
                && tx_languages.contains(&Language::PlutusV3)
            {
                hex::decode(
                    "a1029901291a000189b41901a401011903e818ad00011903e819ea350401192baf18201a000312591920a404193e801864193e801864193e801864193e801864193e801864193e80186418641864193e8018641a000170a718201a00020782182019f016041a0001194a18b2000119568718201a0001643519030104021a00014f581a0001e143191c893903831906b419022518391a00014f580001011903e819a7a90402195fe419733a1826011a000db464196a8f0119ca3f19022e011999101903e819ecb2011a00022a4718201a000144ce1820193bc318201a0001291101193371041956540a197147184a01197147184a0119a9151902280119aecd19021d0119843c18201a00010a9618201a00011aaa1820191c4b1820191cdf1820192d1a18201a00014f581a0001e143191c893903831906b419022518391a00014f5800011a0001614219020700011a000122c118201a00014f581a0001e143191c893903831906b419022518391a00014f580001011a00014f581a0001e143191c893903831906b419022518391a00014f5800011a000e94721a0003414000021a0004213c19583c041a00163cad19fc3604194ff30104001a00022aa818201a000189b41901a401011a00013eff182019e86a1820194eae182019600c1820195108182019654d182019602f18201a0290f1e70a1a032e93af1937fd0a1a0298e40b1966c40a193e801864193e8018641a000eaf1f121a002a6e06061a0006be98011a0321aac7190eac121a00041699121a048e466e1922a4121a0327ec9a121a001e743c18241a0031410f0c1a000dbf9e011a09f2f6d31910d318241a0004578218241a096e44021967b518241a0473cee818241a13e62472011a0f23d40118481a00212c5618481a0022814619fc3b041a00032b00192076041a0013be0419702c183f00011a000f59d919aa6718fb00011a000187551902d61902cf00011a000187551902d61902cf00011a000187551902d61902cf00011a0001a5661902a800011a00017468011a00044a391949a000011a0002bfe2189f01011a00026b371922ee00011a00026e9219226d00011a0001a3e2190ce2011a00019e4919028f011a001df8bb195fc803"
                ).unwrap()
            } else {
                hex::decode(
                    "a241005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0374f693194a1f0aff0198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0223accc0a1a0374f693194a1f0a1a02515e841980b30a"
                ).unwrap()
            }
        }
    } else if network_magic == 2 && network_id == 0 {
        // Preview
        if *block_slot < 777610 {
            // Up to end of epoch 8
            hex::decode(
                "a141005901d59f1a000302590001011a00060bc719026d00011a000249f01903e800011a000249f018201a0025cea81971f70419744d186419744d186419744d186419744d186419744d186419744d18641864186419744d18641a000249f018201a000249f018201a000249f018201a000249f01903e800011a000249f018201a000249f01903e800081a000242201a00067e2318760001011a000249f01903e800081a000249f01a0001b79818f7011a000249f0192710011a0002155e19052e011903e81a000249f01903e8011a000249f018201a000249f018201a000249f0182001011a000249f0011a000249f0041a000194af18f8011a000194af18f8011a0002377c190556011a0002bdea1901f1011a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000242201a00067e23187600010119f04c192bd200011a000249f018201a000242201a00067e2318760001011a000242201a00067e2318760001011a0025cea81971f704001a000141bb041a000249f019138800011a000249f018201a000302590001011a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a00330da70101ff"
            ).unwrap()
        } else if (777610..1900893).contains(block_slot) {
            // From start of epoch 9 up to end of epoch 21
            if tx_languages.contains(&Language::PlutusV1)
                && !tx_languages.contains(&Language::PlutusV2)
            {
                hex::decode(
                    "a141005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a009063b91903fd0aff"
                ).unwrap()
            } else if !tx_languages.contains(&Language::PlutusV1)
                && tx_languages.contains(&Language::PlutusV2)
            {
                hex::decode(
                    "a10198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201b00000004a817c8001b00000004a817c8001a009063b91903fd0a1b00000004a817c800001b00000004a817c800"
                ).unwrap()
            } else {
                hex::decode(
                    "a241005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a009063b91903fd0aff0198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201b00000004a817c8001b00000004a817c8001a009063b91903fd0a1b00000004a817c800001b00000004a817c800"
                ).unwrap()
            }
        } else if (1900893..=9244810).contains(block_slot) {
            // From start of epoch 22 up to end of epoch 106
            if tx_languages.contains(&Language::PlutusV1)
                && !tx_languages.contains(&Language::PlutusV2)
            {
                hex::decode(
                    "a141005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a009063b91903fd0aff"
                ).unwrap()
            } else if !tx_languages.contains(&Language::PlutusV1)
                && tx_languages.contains(&Language::PlutusV2)
            {
                hex::decode(
                    "a10198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0223accc0a1a009063b91903fd0a1a02515e841980b30a"
                ).unwrap()
            } else {
                hex::decode(
                    "a241005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a009063b91903fd0aff0198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0223accc0a1a009063b91903fd0a1a02515e841980b30a"
                ).unwrap()
            }
        } else {
            // From start of epoch 107 onwards
            if tx_languages.contains(&Language::PlutusV1)
                && !tx_languages.contains(&Language::PlutusV2)
                && !tx_languages.contains(&Language::PlutusV3)
            {
                hex::decode(
                    "a141005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0374f693194a1f0aff"
                ).unwrap()
            } else if !tx_languages.contains(&Language::PlutusV1)
                && tx_languages.contains(&Language::PlutusV2)
                && !tx_languages.contains(&Language::PlutusV3)
            {
                hex::decode(
                    "a10198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0223accc0a1a0374f693194a1f0a1a02515e841980b30a"
                ).unwrap()
            } else if !tx_languages.contains(&Language::PlutusV1)
                && !tx_languages.contains(&Language::PlutusV2)
                && tx_languages.contains(&Language::PlutusV3)
            {
                hex::decode(
                    "a1029901291a000189b41901a401011903e818ad00011903e819ea350401192baf18201a000312591920a404193e801864193e801864193e801864193e801864193e801864193e80186418641864193e8018641a000170a718201a00020782182019f016041a0001194a18b2000119568718201a0001643519030104021a00014f581a0001e143191c893903831906b419022518391a00014f580001011903e819a7a90402195fe419733a1826011a000db464196a8f0119ca3f19022e011999101903e819ecb2011a00022a4718201a000144ce1820193bc318201a0001291101193371041956540a197147184a01197147184a0119a9151902280119aecd19021d0119843c18201a00010a9618201a00011aaa1820191c4b1820191cdf1820192d1a18201a00014f581a0001e143191c893903831906b419022518391a00014f5800011a0001614219020700011a000122c118201a00014f581a0001e143191c893903831906b419022518391a00014f580001011a00014f581a0001e143191c893903831906b419022518391a00014f5800011a000e94721a0003414000021a0004213c19583c041a00163cad19fc3604194ff30104001a00022aa818201a000189b41901a401011a00013eff182019e86a1820194eae182019600c1820195108182019654d182019602f18201a0290f1e70a1a032e93af1937fd0a1a0298e40b1966c40a193e801864193e8018641a000eaf1f121a002a6e06061a0006be98011a0321aac7190eac121a00041699121a048e466e1922a4121a0327ec9a121a001e743c18241a0031410f0c1a000dbf9e011a09f2f6d31910d318241a0004578218241a096e44021967b518241a0473cee818241a13e62472011a0f23d40118481a00212c5618481a0022814619fc3b041a00032b00192076041a0013be0419702c183f00011a000f59d919aa6718fb00011a000187551902d61902cf00011a000187551902d61902cf00011a000187551902d61902cf00011a0001a5661902a800011a00017468011a00044a391949a000011a0002bfe2189f01011a00026b371922ee00011a00026e9219226d00011a0001a3e2190ce2011a00019e4919028f011a001df8bb195fc803"
                ).unwrap()
            } else {
                hex::decode(
                    "a241005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0374f693194a1f0aff0198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0223accc0a1a0374f693194a1f0a1a02515e841980b30a"
                ).unwrap()
            }
        }
    } else {
        // All other combinations are assumed to correspond to a mainnet network
        if *block_slot < 72748820 {
            hex::decode(
                "a141005901d59f1a000302590001011a00060bc719026d00011a000249f01903e800011a000249f018201a0025cea81971f70419744d186419744d186419744d186419744d186419744d186419744d18641864186419744d18641a000249f018201a000249f018201a000249f018201a000249f01903e800011a000249f018201a000249f01903e800081a000242201a00067e2318760001011a000249f01903e800081a000249f01a0001b79818f7011a000249f0192710011a0002155e19052e011903e81a000249f01903e8011a000249f018201a000249f018201a000249f0182001011a000249f0011a000249f0041a000194af18f8011a000194af18f8011a0002377c190556011a0002bdea1901f1011a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000242201a00067e23187600010119f04c192bd200011a000249f018201a000242201a00067e2318760001011a000242201a00067e2318760001011a0025cea81971f704001a000141bb041a000249f019138800011a000249f018201a000302590001011a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a00330da70101ff"
            ).unwrap()
        } else if (72748820..84844885).contains(block_slot) {
            // Prior to epoch 394
            if tx_languages.contains(&Language::PlutusV1)
                && !tx_languages.contains(&Language::PlutusV2)
            {
                hex::decode(
                    "a141005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a009063b91903fd0aff"
                ).unwrap()
            } else if !tx_languages.contains(&Language::PlutusV1)
                && tx_languages.contains(&Language::PlutusV2)
            {
                hex::decode(
                    "a10198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201b00000004a817c8001b00000004a817c8001a009063b91903fd0a1b00000004a817c800001b00000004a817c800"
                ).unwrap()
            } else {
                // Precondition allows us to conclude both PlutusV1 and PlutusV2 are required by
                // the transaction
                hex::decode(
                    "a241005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a009063b91903fd0aff0198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201b00000004a817c8001b00000004a817c8001a009063b91903fd0a1b00000004a817c800001b00000004a817c800"
                ).unwrap()
            }
        } else {
            // Starting from epoch 394
            if tx_languages.contains(&Language::PlutusV1)
                && !tx_languages.contains(&Language::PlutusV2)
                && !tx_languages.contains(&Language::PlutusV3)
            {
                hex::decode(
                    "a141005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0374f693194a1f0aff"
                ).unwrap()
            } else if !tx_languages.contains(&Language::PlutusV1)
                && tx_languages.contains(&Language::PlutusV2)
                && !tx_languages.contains(&Language::PlutusV3)
            {
                hex::decode(
                    "a10198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0223accc0a1a0374f693194a1f0a1a02515e841980b30a"
                ).unwrap()
            } else if !tx_languages.contains(&Language::PlutusV1)
                && !tx_languages.contains(&Language::PlutusV2)
                && tx_languages.contains(&Language::PlutusV3)
            {
                hex::decode(
                    "a1029901291a000189b41901a401011903e818ad00011903e819ea350401192baf18201a000312591920a404193e801864193e801864193e801864193e801864193e801864193e80186418641864193e8018641a000170a718201a00020782182019f016041a0001194a18b2000119568718201a0001643519030104021a00014f581a0001e143191c893903831906b419022518391a00014f580001011903e819a7a90402195fe419733a1826011a000db464196a8f0119ca3f19022e011999101903e819ecb2011a00022a4718201a000144ce1820193bc318201a0001291101193371041956540a197147184a01197147184a0119a9151902280119aecd19021d0119843c18201a00010a9618201a00011aaa1820191c4b1820191cdf1820192d1a18201a00014f581a0001e143191c893903831906b419022518391a00014f5800011a0001614219020700011a000122c118201a00014f581a0001e143191c893903831906b419022518391a00014f580001011a00014f581a0001e143191c893903831906b419022518391a00014f5800011a000e94721a0003414000021a0004213c19583c041a00163cad19fc3604194ff30104001a00022aa818201a000189b41901a401011a00013eff182019e86a1820194eae182019600c1820195108182019654d182019602f18201a0290f1e70a1a032e93af1937fd0a1a0298e40b1966c40a193e801864193e8018641a000eaf1f121a002a6e06061a0006be98011a0321aac7190eac121a00041699121a048e466e1922a4121a0327ec9a121a001e743c18241a0031410f0c1a000dbf9e011a09f2f6d31910d318241a0004578218241a096e44021967b518241a0473cee818241a13e62472011a0f23d40118481a00212c5618481a0022814619fc3b041a00032b00192076041a0013be0419702c183f00011a000f59d919aa6718fb00011a000187551902d61902cf00011a000187551902d61902cf00011a000187551902d61902cf00011a0001a5661902a800011a00017468011a00044a391949a000011a0002bfe2189f01011a00026b371922ee00011a00026e9219226d00011a0001a3e2190ce2011a00019e4919028f011a001df8bb195fc803"
                ).unwrap()
            } else {
                hex::decode(
                    "a241005901b69f1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0374f693194a1f0aff0198af1a0003236119032c01011903e819023b00011903e8195e7104011903e818201a0001ca761928eb041959d818641959d818641959d818641959d818641959d818641959d81864186418641959d81864194c5118201a0002acfa182019b551041a000363151901ff00011a00015c3518201a000797751936f404021a0002ff941a0006ea7818dc0001011903e8196ff604021a0003bd081a00034ec5183e011a00102e0f19312a011a00032e801901a5011a0002da781903e819cf06011a00013a34182019a8f118201903e818201a00013aac0119e143041903e80a1a00030219189c011a00030219189c011a0003207c1901d9011a000330001901ff0119ccf3182019fd40182019ffd5182019581e18201940b318201a00012adf18201a0002ff941a0006ea7818dc0001011a00010f92192da7000119eabb18201a0002ff941a0006ea7818dc0001011a0002ff941a0006ea7818dc0001011a0011b22c1a0005fdde00021a000c504e197712041a001d6af61a0001425b041a00040c660004001a00014fab18201a0003236119032c010119a0de18201a00033d7618201979f41820197fb8182019a95d1820197df718201995aa18201a0223accc0a1a0374f693194a1f0a1a02515e841980b30a"
                ).unwrap()
            }
        }
    }
}

fn option_vec_is_empty<T>(option_vec: &Option<Vec<T>>) -> bool {
    match option_vec {
        Some(vec) => vec.is_empty(),
        None => true,
    }
}
