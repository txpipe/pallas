//! Utilities required for Alonzo-era transaction validation.

use crate::utils::{
    add_minted_value, add_values, aux_data_from_alonzo_minted_tx, compute_native_script_hash,
    compute_plutus_v1_script_hash, empty_value, get_alonzo_comp_tx_size,
    get_lovelace_from_alonzo_val, get_payment_part, get_shelley_address, get_val_size_in_words,
    mk_alonzo_vk_wits_check_list, values_are_equal, verify_signature,
    AlonzoError::*,
    AlonzoProtParams, UTxOs,
    ValidationError::{self, *},
    ValidationResult,
};
use hex;
use pallas_addresses::{ScriptHash, ShelleyAddress, ShelleyPaymentPart};
use pallas_codec::{
    minicbor::{encode, Encoder},
    utils::{Bytes, KeepRaw},
};
use pallas_crypto::hash::Hash;
use pallas_primitives::{
    alonzo::{
        AddrKeyhash, Mint, Multiasset, NativeScript, PlutusData, PlutusScript, PolicyId, Redeemer,
        RedeemerPointer, RedeemerTag, RequiredSigners, TransactionBody, TransactionInput,
        TransactionOutput, Tx, VKeyWitness, Value, WitnessSet,
    },
    byron::TxOut,
};
use pallas_traverse::{MultiEraInput, MultiEraOutput, OriginalHash};
use std::ops::Deref;

pub fn validate_alonzo_tx(
    mtx: &Tx,
    utxos: &UTxOs,
    prot_pps: &AlonzoProtParams,
    block_slot: &u64,
    network_id: &u8,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body;
    let size: u32 = get_alonzo_comp_tx_size(mtx);
    check_ins_not_empty(tx_body)?;
    check_ins_and_collateral_in_utxos(tx_body, utxos)?;
    check_tx_validity_interval(tx_body, mtx, block_slot)?;
    check_fee(tx_body, &size, mtx, utxos, prot_pps)?;
    check_preservation_of_value(tx_body, utxos)?;
    check_min_lovelace(tx_body, prot_pps)?;
    check_output_val_size(tx_body, prot_pps)?;
    check_network_id(tx_body, network_id)?;
    check_tx_size(&size, prot_pps)?;
    check_tx_ex_units(mtx, prot_pps)?;
    check_witness_set(mtx, utxos)?;
    check_languages(mtx, prot_pps)?;
    check_auxiliary_data(tx_body, mtx)?;
    check_script_data_hash(tx_body, mtx)?;
    check_minting(tx_body, mtx)
}

// The set of transaction inputs is not empty.
fn check_ins_not_empty(tx_body: &TransactionBody) -> ValidationResult {
    if tx_body.inputs.is_empty() {
        return Err(Alonzo(TxInsEmpty));
    }
    Ok(())
}

// All transaction inputs and collateral inputs are in the set of (yet) unspent
// transaction outputs.
fn check_ins_and_collateral_in_utxos(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    for input in tx_body.inputs.iter() {
        if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(input))) {
            return Err(Alonzo(InputNotInUTxO));
        }
    }
    match &tx_body.collateral {
        None => Ok(()),
        Some(collaterals) => {
            for collateral in collaterals {
                if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(collateral))) {
                    return Err(Alonzo(CollateralNotInUTxO));
                }
            }
            Ok(())
        }
    }
}

// The block slot is contained in the transaction validity interval, and the
// upper bound is translatable to UTC time.
fn check_tx_validity_interval(
    tx_body: &TransactionBody,
    mtx: &Tx,
    block_slot: &u64,
) -> ValidationResult {
    check_lower_bound(tx_body, block_slot)?;
    check_upper_bound(tx_body, mtx, block_slot)
}

// If defined, the lower bound of the validity time interval does not exceed the
// block slot.
fn check_lower_bound(tx_body: &TransactionBody, block_slot: &u64) -> ValidationResult {
    match tx_body.validity_interval_start {
        Some(lower_bound) => {
            if *block_slot < lower_bound {
                Err(Alonzo(BlockPrecedesValInt))
            } else {
                Ok(())
            }
        }
        None => Ok(()),
    }
}

// If defined, the upper bound of the validity time interval is not exceeded by
// the block slot, and it is translatable to UTC time.
fn check_upper_bound(tx_body: &TransactionBody, _mtx: &Tx, block_slot: &u64) -> ValidationResult {
    match tx_body.ttl {
        Some(upper_bound) => {
            if upper_bound < *block_slot {
                Err(Alonzo(BlockExceedsValInt))
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
    prot_pps: &AlonzoProtParams,
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
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    if tx_body.fee < (prot_pps.minfee_b + prot_pps.minfee_a * size) as u64 {
        return Err(Alonzo(FeeBelowMin));
    }
    Ok(())
}

fn presence_of_plutus_scripts(mtx: &Tx) -> bool {
    let minted_witness_set: &WitnessSet = &mtx.transaction_witness_set;
    match &minted_witness_set.plutus_script {
        Some(plutus_v1_scripts) => !plutus_v1_scripts.is_empty(),
        None => false,
    }
}

fn check_collaterals(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    let collaterals: &Vec<TransactionInput> = &tx_body
        .collateral
        .clone()
        .ok_or(Alonzo(CollateralMissing))?;
    check_collaterals_number(collaterals, prot_pps)?;
    check_collaterals_address(collaterals, utxos)?;
    check_collaterals_assets(tx_body, utxos, prot_pps)
}

// The set of collateral inputs is not empty.
// The number of collateral inputs is below maximum allowed by protocol.
fn check_collaterals_number(
    collaterals: &[TransactionInput],
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    let number_collateral: u32 = collaterals.len() as u32;
    if number_collateral == 0 {
        Err(Alonzo(CollateralMissing))
    } else if number_collateral > prot_pps.max_collateral_inputs {
        Err(Alonzo(TooManyCollaterals))
    } else {
        Ok(())
    }
}

// Each collateral input refers to a verification-key address.
fn check_collaterals_address(collaterals: &[TransactionInput], utxos: &UTxOs) -> ValidationResult {
    for collateral in collaterals {
        match utxos.get(&MultiEraInput::from_alonzo_compatible(collateral)) {
            Some(multi_era_output) => {
                if let Some(alonzo_comp_output) = MultiEraOutput::as_alonzo(multi_era_output) {
                    if let ShelleyPaymentPart::Script(_) =
                        get_payment_part(&alonzo_comp_output.address)
                            .ok_or(Alonzo(InputDecoding))?
                    {
                        return Err(Alonzo(CollateralNotVKeyLocked));
                    }
                }
            }
            None => return Err(Alonzo(CollateralNotInUTxO)),
        }
    }
    Ok(())
}

// Collateral inputs contain only lovelace, and in a number not lower than the
// minimum allowed.
fn check_collaterals_assets(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    let fee_percentage: u64 = tx_body.fee * prot_pps.collateral_percentage as u64;
    match &tx_body.collateral {
        Some(collaterals) => {
            for collateral in collaterals {
                match utxos.get(&MultiEraInput::from_alonzo_compatible(collateral)) {
                    Some(multi_era_output) => match MultiEraOutput::as_alonzo(multi_era_output) {
                        Some(TransactionOutput {
                            amount: Value::Coin(n),
                            ..
                        }) => {
                            if *n * 100 < fee_percentage {
                                return Err(Alonzo(CollateralMinLovelace));
                            }
                        }
                        Some(TransactionOutput {
                            amount: Value::Multiasset(n, multi_assets),
                            ..
                        }) => {
                            if *n * 100 < fee_percentage {
                                return Err(Alonzo(CollateralMinLovelace));
                            }
                            if !multi_assets.is_empty() {
                                return Err(Alonzo(NonLovelaceCollateral));
                            }
                        }
                        None => (),
                    },
                    None => return Err(Alonzo(CollateralNotInUTxO)),
                }
            }
        }
        None => return Err(Alonzo(CollateralMissing)),
    }
    Ok(())
}

// The preservation of value property holds.
fn check_preservation_of_value(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    let mut input: Value = get_consumed(tx_body, utxos)?;
    let produced: Value = get_produced(tx_body)?;
    let output: Value = add_values(&produced, &Value::Coin(tx_body.fee), &Alonzo(NegativeValue))?;
    if let Some(m) = &tx_body.mint {
        input = add_minted_value(&input, m, &Alonzo(NegativeValue))?;
    }
    if !values_are_equal(&input, &output) {
        return Err(Alonzo(PreservationOfValue));
    }
    Ok(())
}

fn get_consumed(tx_body: &TransactionBody, utxos: &UTxOs) -> Result<Value, ValidationError> {
    let mut res: Value = empty_value();
    for input in tx_body.inputs.iter() {
        let utxo_value: &MultiEraOutput = utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .ok_or(Alonzo(InputNotInUTxO))?;
        match MultiEraOutput::as_alonzo(utxo_value) {
            Some(TransactionOutput { amount, .. }) => {
                res = add_values(&res, amount, &Alonzo(NegativeValue))?
            }
            None => match MultiEraOutput::as_byron(utxo_value) {
                Some(TxOut { amount, .. }) => {
                    res = add_values(&res, &Value::Coin(*amount), &Alonzo(NegativeValue))?
                }
                _ => return Err(Alonzo(InputNotInUTxO)),
            },
        }
    }
    Ok(res)
}

fn get_produced(tx_body: &TransactionBody) -> Result<Value, ValidationError> {
    let mut res: Value = empty_value();
    for TransactionOutput { amount, .. } in tx_body.outputs.iter() {
        res = add_values(&res, amount, &Alonzo(NegativeValue))?;
    }
    Ok(res)
}

// All transaction outputs should contain at least the minimum lovelace.
fn check_min_lovelace(tx_body: &TransactionBody, prot_pps: &AlonzoProtParams) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        if get_lovelace_from_alonzo_val(&output.amount) < compute_min_lovelace(output, prot_pps) {
            return Err(Alonzo(MinLovelaceUnreached));
        }
    }
    Ok(())
}

fn compute_min_lovelace(output: &TransactionOutput, prot_pps: &AlonzoProtParams) -> u64 {
    let output_entry_size: u64 = get_val_size_in_words(&output.amount)
        + match output.datum_hash {
            Some(_) => 37, // utxoEntrySizeWithoutVal (27) + dataHashSize (10)
            None => 27,    // utxoEntrySizeWithoutVal
        };
    prot_pps.ada_per_utxo_byte * output_entry_size
}

// The size of the value in each of the outputs should not be greater than the
// maximum allowed.
fn check_output_val_size(
    tx_body: &TransactionBody,
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        if get_val_size_in_words(&output.amount) > prot_pps.max_value_size as u64 {
            return Err(Alonzo(MaxValSizeExceeded));
        }
    }
    Ok(())
}

// The network ID of the transaction and its output addresses is correct.
fn check_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    check_tx_outs_network_id(tx_body, network_id)?;
    check_tx_network_id(tx_body, network_id)
}

// The network ID of each output matches the global network ID.
fn check_tx_outs_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let addr: ShelleyAddress =
            get_shelley_address(Bytes::deref(&output.address)).ok_or(Alonzo(AddressDecoding))?;
        if addr.network().value() != *network_id {
            return Err(Alonzo(OutputWrongNetworkID));
        }
    }
    Ok(())
}

// The network ID of the transaction body is either undefined or equal to the
// global network ID.
fn check_tx_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    if let Some(tx_network_id) = tx_body.network_id {
        if u8::from(tx_network_id) != *network_id {
            return Err(Alonzo(TxWrongNetworkID));
        }
    }
    Ok(())
}

// The transaction size does not exceed the protocol limit.
fn check_tx_size(size: &u32, prot_pps: &AlonzoProtParams) -> ValidationResult {
    if *size > prot_pps.max_transaction_size {
        return Err(Alonzo(MaxTxSizeExceeded));
    }
    Ok(())
}

// The number of execution units of the transaction should not exceed the
// maximum allowed.
fn check_tx_ex_units(mtx: &Tx, prot_pps: &AlonzoProtParams) -> ValidationResult {
    let tx_wits: &WitnessSet = &mtx.transaction_witness_set;
    if presence_of_plutus_scripts(mtx) {
        match &tx_wits.redeemer {
            Some(redeemers_vec) => {
                let mut steps: u64 = 0;
                let mut mem: u64 = 0;
                for Redeemer { ex_units, .. } in redeemers_vec {
                    mem += ex_units.mem;
                    steps += ex_units.steps;
                }
                if mem > prot_pps.max_tx_ex_units.mem || steps > prot_pps.max_tx_ex_units.steps {
                    return Err(Alonzo(TxExUnitsExceeded));
                }
            }
            None => return Err(Alonzo(RedeemerMissing)),
        }
    }
    Ok(())
}

fn check_witness_set(mtx: &Tx, utxos: &UTxOs) -> ValidationResult {
    let tx_hash: &Vec<u8> = &Vec::from(mtx.transaction_body.original_hash().as_ref());
    let tx_body: &TransactionBody = &mtx.transaction_body;
    let tx_wits: &WitnessSet = &mtx.transaction_witness_set;
    let vkey_wits: &Option<Vec<VKeyWitness>> = &tx_wits.vkeywitness;
    let native_scripts: Vec<NativeScript> = match &tx_wits.native_script {
        Some(scripts) => scripts.clone().iter().map(|x| x.clone().unwrap()).collect(),
        None => Vec::new(),
    };
    let plutus_v1_scripts: Vec<PlutusScript<1>> = match &tx_wits.plutus_script {
        Some(scripts) => scripts.clone(),
        None => Vec::new(),
    };
    check_needed_scripts_are_included(tx_body, utxos, &native_scripts, &plutus_v1_scripts)?;
    check_datums(tx_body, utxos, &tx_wits.plutus_data)?;
    check_redeemers(tx_body, tx_wits, utxos)?;
    check_required_signers(&tx_body.required_signers, vkey_wits, tx_hash)?;
    check_vkey_input_wits(mtx, &tx_wits.vkeywitness, utxos)
}

// The set of needed scripts (minting policies, native scripts and Plutus
// scripts needed to validate the transaction) equals the set of scripts
// contained in the transaction witnesses set.
fn check_needed_scripts_are_included(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    native_scripts: &[NativeScript],
    plutus_v1_scripts: &[PlutusScript<1>],
) -> ValidationResult {
    let mut native_scripts: Vec<(bool, NativeScript)> =
        native_scripts.iter().map(|x| (false, x.clone())).collect();
    let mut plutus_v1_scripts: Vec<(bool, PlutusScript<1>)> = plutus_v1_scripts
        .iter()
        .map(|x| (false, x.clone()))
        .collect();
    check_script_inputs(tx_body, &mut native_scripts, &mut plutus_v1_scripts, utxos)?;
    check_minting_policies(tx_body, &mut native_scripts, &mut plutus_v1_scripts)?;
    for (native_script_covered, _) in native_scripts.iter() {
        if !native_script_covered {
            return Err(Alonzo(UnneededNativeScript));
        }
    }
    for (plutus_v1_script_covered, _) in plutus_v1_scripts.iter() {
        if !plutus_v1_script_covered {
            return Err(Alonzo(UnneededPlutusScript));
        }
    }
    Ok(())
}

fn check_datums(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    option_plutus_data: &Option<Vec<KeepRaw<PlutusData>>>,
) -> ValidationResult {
    let mut plutus_data: Vec<(bool, &KeepRaw<PlutusData>)> = match option_plutus_data {
        Some(plutus_data) => plutus_data.iter().map(|x| (false, x)).collect(),
        None => Vec::new(),
    };
    check_input_datum_hash_in_witness_set(tx_body, utxos, &mut plutus_data)?;
    check_datums_from_witness_set_in_inputs_or_outputs(tx_body, &plutus_data)
}

// Each datum hash in a Plutus script input matches the hash of a datum in the
// transaction witness set.
fn check_input_datum_hash_in_witness_set(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    plutus_data: &mut [(bool, &KeepRaw<PlutusData>)],
) -> ValidationResult {
    for input in &tx_body.inputs {
        match utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .and_then(MultiEraOutput::as_alonzo)
        {
            Some(output) => {
                if let Some(datum_hash) = output.datum_hash {
                    find_datum_hash(datum_hash, plutus_data)?
                }
            }
            None => return Err(Alonzo(InputNotInUTxO)),
        }
    }
    Ok(())
}

fn find_datum_hash(
    datum_hash: Hash<32>,
    plutus_data: &mut [(bool, &KeepRaw<PlutusData>)],
) -> ValidationResult {
    for (found, datum) in plutus_data {
        let computed_datum_hash = pallas_crypto::hash::Hasher::<256>::hash(datum.raw_cbor());
        if datum_hash == computed_datum_hash {
            *found = true;
            return Ok(());
        }
    }
    Err(Alonzo(DatumMissing))
}

// Each datum object in the transaction witness set corresponds either to an
// output datum hash or to the datum hash of a Plutus script input.
fn check_datums_from_witness_set_in_inputs_or_outputs(
    tx_body: &TransactionBody,
    plutus_data: &[(bool, &KeepRaw<PlutusData>)],
) -> ValidationResult {
    for (found, datum) in plutus_data {
        if !found {
            find_datum(datum, &tx_body.outputs)?
        }
    }
    Ok(())
}

fn find_datum(datum: &KeepRaw<PlutusData>, outputs: &[TransactionOutput]) -> ValidationResult {
    for output in outputs {
        if let Some(hash) = output.datum_hash {
            if pallas_crypto::hash::Hasher::<256>::hash(datum.raw_cbor()) == hash {
                return Ok(());
            }
        }
    }
    Err(Alonzo(UnneededDatum))
}

// The set of redeemers in the transaction witness set should match the set of
// Plutus scripts needed to validate the transaction.
fn check_redeemers(
    tx_body: &TransactionBody,
    tx_wits: &WitnessSet,
    utxos: &UTxOs,
) -> ValidationResult {
    let redeemer_pointers: Vec<RedeemerPointer> = match &tx_wits.redeemer {
        Some(redeemers) => redeemers
            .iter()
            .map(|x| RedeemerPointer {
                tag: x.tag,
                index: x.index,
            })
            .collect(),
        None => Vec::new(),
    };
    let plutus_scripts: Vec<RedeemerPointer> = mk_plutus_script_redeemer_pointers(
        &sort_inputs(&tx_body.inputs),
        &tx_body.mint,
        tx_wits,
        utxos,
    );
    redeemer_pointers_coincide(&redeemer_pointers, &plutus_scripts)
}

fn mk_plutus_script_redeemer_pointers(
    sorted_inputs: &[TransactionInput],
    mint: &Option<Multiasset<i64>>,
    tx_wits: &WitnessSet,
    utxos: &UTxOs,
) -> Vec<RedeemerPointer> {
    match &tx_wits.plutus_script {
        Some(plutus_scripts) => {
            let mut res: Vec<RedeemerPointer> = Vec::new();
            for (index, input) in sorted_inputs.iter().enumerate() {
                if let Some(script_hash) = get_script_hash_from_input(input, utxos) {
                    for plutus_script in plutus_scripts.iter() {
                        let hashed_script: PolicyId = compute_plutus_v1_script_hash(plutus_script);
                        if script_hash == hashed_script {
                            res.push(RedeemerPointer {
                                tag: RedeemerTag::Spend,
                                index: index as u32,
                            })
                        }
                    }
                }
            }

            if let Some(minted_value) = mint {
                let sorted_policies: Vec<PolicyId> = sort_policies(minted_value);
                for (index, policy) in sorted_policies.iter().enumerate() {
                    for plutus_script in plutus_scripts.iter() {
                        let hashed_script: PolicyId = compute_plutus_v1_script_hash(plutus_script);
                        if *policy == hashed_script {
                            res.push(RedeemerPointer {
                                tag: RedeemerTag::Mint,
                                index: index as u32,
                            })
                        }
                    }
                }
            }

            res
        }
        None => Vec::new(),
    }
}

// Lexicographical sorting for inputs.
fn sort_inputs(unsorted_inputs: &[TransactionInput]) -> Vec<TransactionInput> {
    let mut res: Vec<TransactionInput> = unsorted_inputs.to_owned();
    res.sort();
    res
}

// Lexicographical sorting for PolicyID's.
fn sort_policies(mint: &Mint) -> Vec<PolicyId> {
    let mut res: Vec<PolicyId> = mint
        .clone()
        .iter()
        .map(|(policy_id, _)| *policy_id)
        .collect();
    res.sort();
    res
}

fn redeemer_pointers_coincide(
    redeemers: &[RedeemerPointer],
    plutus_scripts: &[RedeemerPointer],
) -> ValidationResult {
    for redeemer_pointer in redeemers {
        if plutus_scripts.iter().all(|x| x != redeemer_pointer) {
            return Err(Alonzo(UnneededRedeemer));
        }
    }
    for ps_redeemer_pointer in plutus_scripts {
        if redeemers.iter().all(|x| x != ps_redeemer_pointer) {
            return Err(Alonzo(RedeemerMissing));
        }
    }
    Ok(())
}

fn check_script_inputs(
    tx_body: &TransactionBody,
    native_scripts: &mut [(bool, NativeScript)],
    plutus_v1_scripts: &mut [(bool, PlutusScript<1>)],
    utxos: &UTxOs,
) -> ValidationResult {
    let mut inputs: Vec<(bool, ScriptHash)> = get_script_hashes(tx_body, utxos);
    for (input_script_covered, input_script_hash) in &mut inputs {
        for (native_script_covered, native_script) in native_scripts.iter_mut() {
            let hashed_script: PolicyId = compute_native_script_hash(native_script);
            if *input_script_hash == hashed_script {
                *input_script_covered = true;
                *native_script_covered = true;
            }
        }
        for (plutus_script_covered, plutus_v1_script) in plutus_v1_scripts.iter_mut() {
            let hashed_script: PolicyId = compute_plutus_v1_script_hash(plutus_v1_script);
            if *input_script_hash == hashed_script {
                *input_script_covered = true;
                *plutus_script_covered = true;
            }
        }
    }
    for (input_script_covered, _) in inputs {
        if !input_script_covered {
            return Err(Alonzo(ScriptWitnessMissing));
        }
    }
    Ok(())
}

fn get_script_hashes(tx_body: &TransactionBody, utxos: &UTxOs) -> Vec<(bool, ScriptHash)> {
    let mut res: Vec<(bool, ScriptHash)> = Vec::new();
    for input in tx_body.inputs.iter() {
        if let Some(script_hash) = get_script_hash_from_input(input, utxos) {
            res.push((false, script_hash))
        }
    }
    res
}

fn get_script_hash_from_input(input: &TransactionInput, utxos: &UTxOs) -> Option<ScriptHash> {
    utxos
        .get(&MultiEraInput::from_alonzo_compatible(input))
        .and_then(MultiEraOutput::as_alonzo)
        .and_then(|tx_out| get_payment_part(&tx_out.address))
        .and_then(|payment_part| match payment_part {
            ShelleyPaymentPart::Script(script_hash) => Some(script_hash),
            _ => None,
        })
}

fn check_minting_policies(
    tx_body: &TransactionBody,
    native_scripts: &mut [(bool, NativeScript)],
    plutus_v1_scripts: &mut [(bool, PlutusScript<1>)],
) -> ValidationResult {
    match &tx_body.mint {
        None => Ok(()),
        Some(minted_value) => {
            let mut minting_policies: Vec<(bool, PolicyId)> =
                minted_value.iter().map(|(pol, _)| (false, *pol)).collect();
            for (policy_covered, policy) in &mut minting_policies {
                for (native_script_covered, native_script) in native_scripts.iter_mut() {
                    let hashed_script: PolicyId = compute_native_script_hash(native_script);
                    if *policy == hashed_script {
                        *policy_covered = true;
                        *native_script_covered = true;
                    }
                }
                for (plutus_script_covered, plutus_v1_script) in plutus_v1_scripts.iter_mut() {
                    let hashed_script: PolicyId = compute_plutus_v1_script_hash(plutus_v1_script);
                    if *policy == hashed_script {
                        *policy_covered = true;
                        *plutus_script_covered = true;
                    }
                }
            }
            for (policy_covered, _) in minting_policies {
                if !policy_covered {
                    return Err(Alonzo(MintingLacksPolicy));
                }
            }
            Ok(())
        }
    }
}

// The owner of each transaction input and each collateral input should have
// signed the transaction.
fn check_vkey_input_wits(
    mtx: &Tx,
    vkey_wits: &Option<Vec<VKeyWitness>>,
    utxos: &UTxOs,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body;
    let vk_wits: &mut Vec<(bool, VKeyWitness)> =
        &mut mk_alonzo_vk_wits_check_list(vkey_wits, Alonzo(VKWitnessMissing))?;
    let tx_hash: &Vec<u8> = &Vec::from(mtx.transaction_body.original_hash().as_ref());
    let mut inputs_and_collaterals: Vec<TransactionInput> = Vec::new();
    inputs_and_collaterals.extend(tx_body.inputs.clone());

    if let Some(collaterals) = &tx_body.collateral {
        inputs_and_collaterals.extend(collaterals.clone());
    }

    for input in inputs_and_collaterals.iter() {
        match utxos.get(&MultiEraInput::from_alonzo_compatible(input)) {
            Some(multi_era_output) => {
                if let Some(alonzo_comp_output) = MultiEraOutput::as_alonzo(multi_era_output) {
                    match get_payment_part(&alonzo_comp_output.address)
                        .ok_or(Alonzo(InputDecoding))?
                    {
                        ShelleyPaymentPart::Key(payment_key_hash) => {
                            check_vk_wit(&payment_key_hash, vk_wits, tx_hash)?
                        }
                        ShelleyPaymentPart::Script(_) => (),
                    }
                }
            }
            None => return Err(Alonzo(InputNotInUTxO)),
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
                return Err(Alonzo(VKWrongSignature));
            } else {
                *vkey_wit_covered = true;
                return Ok(());
            }
        }
    }
    Err(Alonzo(VKWitnessMissing))
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
                return Err(Alonzo(VKWrongSignature));
            }
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
            None => return Err(Alonzo(ReqSignerMissing)),
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
                return Err(Alonzo(ReqSignerWrongSig));
            } else {
                return Ok(());
            }
        }
    }
    Err(Alonzo(ReqSignerMissing))
}

// The required script languages are included in the protocol parameters.
fn check_languages(_mtx: &Tx, _prot_pps: &AlonzoProtParams) -> ValidationResult {
    Ok(())
}

// The metadata of the transaction is valid.
fn check_auxiliary_data(tx_body: &TransactionBody, mtx: &Tx) -> ValidationResult {
    match (
        &tx_body.auxiliary_data_hash,
        aux_data_from_alonzo_minted_tx(mtx),
    ) {
        (Some(metadata_hash), Some(metadata)) => {
            if metadata_hash.as_slice()
                == pallas_crypto::hash::Hasher::<256>::hash(metadata).as_ref()
            {
                Ok(())
            } else {
                Err(Alonzo(MetadataHash))
            }
        }
        (None, None) => Ok(()),
        _ => Err(Alonzo(MetadataHash)),
    }
}

// The script data integrity hash matches the hash of the redeemers, languages
// and datums of the transaction witness set.
fn check_script_data_hash(tx_body: &TransactionBody, mtx: &Tx) -> ValidationResult {
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
                if script_data_hash == compute_script_integrity_hash(&plutus_data, redeemer) {
                    Ok(())
                } else {
                    Err(Alonzo(ScriptIntegrityHash))
                }
            }
            (_, _) => Err(Alonzo(ScriptIntegrityHash)),
        },
        None => {
            if option_vec_is_empty(&mtx.transaction_witness_set.plutus_data)
                && option_vec_is_empty(&mtx.transaction_witness_set.redeemer)
            {
                Ok(())
            } else {
                Err(Alonzo(ScriptIntegrityHash))
            }
        }
    }
}

fn compute_script_integrity_hash(plutus_data: &[PlutusData], redeemer: &[Redeemer]) -> Hash<32> {
    let mut value_to_hash: Vec<u8> = Vec::new();
    // First, the Redeemer.
    let _ = encode(redeemer, &mut value_to_hash);
    // Next, the PlutusData.
    let mut plutus_data_encoder: Encoder<Vec<u8>> = Encoder::new(Vec::new());
    let _ = plutus_data_encoder.begin_array();
    for single_plutus_data in plutus_data.iter() {
        let _ = plutus_data_encoder.encode(single_plutus_data);
    }
    let _ = plutus_data_encoder.end();
    value_to_hash.extend(plutus_data_encoder.writer().clone());
    // Finally, the cost model.
    value_to_hash.extend(cost_model_cbor());
    pallas_crypto::hash::Hasher::<256>::hash(&value_to_hash)
}

fn cost_model_cbor() -> Vec<u8> {
    // Mainnet, preprod and preview all have the same cost model during the Alonzo
    // era.
    hex::decode(
        "a141005901d59f1a000302590001011a00060bc719026d00011a000249f01903e800011a000249f018201a0025cea81971f70419744d186419744d186419744d186419744d186419744d186419744d18641864186419744d18641a000249f018201a000249f018201a000249f018201a000249f01903e800011a000249f018201a000249f01903e800081a000242201a00067e2318760001011a000249f01903e800081a000249f01a0001b79818f7011a000249f0192710011a0002155e19052e011903e81a000249f01903e8011a000249f018201a000249f018201a000249f0182001011a000249f0011a000249f0041a000194af18f8011a000194af18f8011a0002377c190556011a0002bdea1901f1011a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000242201a00067e23187600010119f04c192bd200011a000249f018201a000242201a00067e2318760001011a000242201a00067e2318760001011a0025cea81971f704001a000141bb041a000249f019138800011a000249f018201a000302590001011a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a00330da70101ff"
    ).unwrap()
}

fn option_vec_is_empty<T>(option_vec: &Option<Vec<T>>) -> bool {
    match option_vec {
        Some(vec) => vec.is_empty(),
        None => true,
    }
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
            let plutus_v1_script_wits: Vec<PlutusScript<1>> =
                match &mtx.transaction_witness_set.plutus_script {
                    None => Vec::new(),
                    Some(plutus_v1_script_wits) => plutus_v1_script_wits.clone(),
                };
            for (policy, _) in minted_value.iter() {
                if native_script_wits
                    .iter()
                    .all(|native_script| compute_native_script_hash(native_script) != *policy)
                    && plutus_v1_script_wits.iter().all(|plutus_v1_script| {
                        compute_plutus_v1_script_hash(plutus_v1_script) != *policy
                    })
                {
                    return Err(Alonzo(MintingLacksPolicy));
                }
            }
            Ok(())
        }
        None => Ok(()),
    }
}
