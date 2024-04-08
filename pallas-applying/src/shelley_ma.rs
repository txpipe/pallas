//! Utilities required for ShelleyMA-era transaction validation.

use crate::utils::{
    add_minted_value, add_values, aux_data_from_alonzo_minted_tx, empty_value,
    get_alonzo_comp_tx_size, get_lovelace_from_alonzo_val, get_payment_part, get_shelley_address,
    get_val_size_in_words, mk_alonzo_vk_wits_check_list, values_are_equal, verify_signature,
    ShelleyMAError::*,
    ShelleyProtParams, UTxOs,
    ValidationError::{self, *},
    ValidationResult,
};
use pallas_addresses::{PaymentKeyHash, ScriptHash, ShelleyAddress, ShelleyPaymentPart};
use pallas_codec::minicbor::encode;
use pallas_primitives::{
    alonzo::{
        MintedTx, MintedWitnessSet, NativeScript, PolicyId, TransactionBody, TransactionOutput,
        VKeyWitness, Value,
    },
    byron::TxOut,
};
use pallas_traverse::{ComputeHash, Era, MultiEraInput, MultiEraOutput};
use std::{cmp::max, ops::Deref};

pub fn validate_shelley_ma_tx(
    mtx: &MintedTx,
    utxos: &UTxOs,
    prot_pps: &ShelleyProtParams,
    block_slot: &u64,
    network_id: &u8,
    era: &Era,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body;
    let tx_wits: &MintedWitnessSet = &mtx.transaction_witness_set;
    let size: &u32 = &get_alonzo_comp_tx_size(tx_body).ok_or(ShelleyMA(UnknownTxSize))?;
    check_ins_not_empty(tx_body)?;
    check_ins_in_utxos(tx_body, utxos)?;
    check_ttl(tx_body, block_slot)?;
    check_tx_size(size, prot_pps)?;
    check_min_lovelace(tx_body, prot_pps, era)?;
    check_preservation_of_value(tx_body, utxos, era)?;
    check_fees(tx_body, size, prot_pps)?;
    check_network_id(tx_body, network_id)?;
    check_metadata(tx_body, mtx)?;
    check_witnesses(tx_body, tx_wits, utxos)?;
    check_minting(tx_body, mtx)
}

fn check_ins_not_empty(tx_body: &TransactionBody) -> ValidationResult {
    if tx_body.inputs.is_empty() {
        return Err(ShelleyMA(TxInsEmpty));
    }
    Ok(())
}

fn check_ins_in_utxos(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    for input in tx_body.inputs.iter() {
        if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(input))) {
            return Err(ShelleyMA(InputNotInUTxO));
        }
    }
    Ok(())
}

fn check_ttl(tx_body: &TransactionBody, block_slot: &u64) -> ValidationResult {
    match tx_body.ttl {
        Some(ttl) => {
            if ttl < *block_slot {
                Err(ShelleyMA(TTLExceeded))
            } else {
                Ok(())
            }
        }
        None => Err(ShelleyMA(AlonzoCompNotShelley)),
    }
}

fn check_tx_size(size: &u32, prot_pps: &ShelleyProtParams) -> ValidationResult {
    if *size > prot_pps.max_transaction_size {
        return Err(ShelleyMA(MaxTxSizeExceeded));
    }
    Ok(())
}

fn check_min_lovelace(
    tx_body: &TransactionBody,
    prot_pps: &ShelleyProtParams,
    era: &Era,
) -> ValidationResult {
    for output in &tx_body.outputs {
        match era {
            Era::Shelley | Era::Allegra | Era::Mary => {
                if get_lovelace_from_alonzo_val(&output.amount)
                    < compute_min_lovelace(output, prot_pps)
                {
                    return Err(ShelleyMA(MinLovelaceUnreached));
                }
            }
            _ => return Err(ShelleyMA(ValueNotShelley)),
        }
    }
    Ok(())
}

fn compute_min_lovelace(output: &TransactionOutput, prot_pps: &ShelleyProtParams) -> u64 {
    match &output.amount {
        Value::Coin(_) => prot_pps.min_utxo_value,
        Value::Multiasset(lovelace, _) => {
            let utxo_entry_size: u64 = 27 + get_val_size_in_words(&output.amount);
            let coins_per_utxo_word: u64 = prot_pps.min_utxo_value / 27;
            max(*lovelace, utxo_entry_size * coins_per_utxo_word)
        }
    }
}

fn check_preservation_of_value(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    era: &Era,
) -> ValidationResult {
    let neg_val_err: ValidationError = ShelleyMA(NegativeValue);
    let input: Value = get_consumed(tx_body, utxos, era)?;
    let produced: Value = get_produced(tx_body, era)?;
    let output: Value = add_values(&produced, &Value::Coin(tx_body.fee), &neg_val_err)?;
    if let Some(m) = &tx_body.mint {
        add_minted_value(&output, m, &neg_val_err)?;
    }
    if !values_are_equal(&input, &output) {
        return Err(ShelleyMA(PreservationOfValue));
    }
    Ok(())
}

fn get_consumed(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    era: &Era,
) -> Result<Value, ValidationError> {
    let neg_val_err: ValidationError = ShelleyMA(NegativeValue);
    let mut res: Value = empty_value();
    for input in tx_body.inputs.iter() {
        let utxo_value: &MultiEraOutput = utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .ok_or(ShelleyMA(InputNotInUTxO))?;
        match MultiEraOutput::as_alonzo(utxo_value) {
            Some(TransactionOutput { amount, .. }) => match (amount, era) {
                (Value::Coin(..), _) => res = add_values(&res, amount, &neg_val_err)?,
                (Value::Multiasset(..), Era::Shelley) => return Err(ShelleyMA(ValueNotShelley)),
                _ => res = add_values(&res, amount, &neg_val_err)?,
            },
            None => match MultiEraOutput::as_byron(utxo_value) {
                Some(TxOut { amount, .. }) => {
                    res = add_values(&res, &Value::Coin(*amount), &neg_val_err)?
                }
                _ => return Err(ShelleyMA(InputNotInUTxO)),
            },
        }
    }
    Ok(res)
}

fn get_produced(tx_body: &TransactionBody, era: &Era) -> Result<Value, ValidationError> {
    let neg_val_err: ValidationError = ShelleyMA(NegativeValue);
    let mut res: Value = empty_value();
    for TransactionOutput { amount, .. } in tx_body.outputs.iter() {
        match (amount, era) {
            (Value::Coin(..), _) => res = add_values(&res, amount, &neg_val_err)?,
            (Value::Multiasset(..), Era::Shelley) => return Err(ShelleyMA(WrongEraOutput)),
            _ => res = add_values(&res, amount, &neg_val_err)?,
        }
    }
    Ok(res)
}

fn check_fees(
    tx_body: &TransactionBody,
    size: &u32,
    prot_pps: &ShelleyProtParams,
) -> ValidationResult {
    if tx_body.fee < (prot_pps.minfee_b + prot_pps.minfee_a * size) as u64 {
        return Err(ShelleyMA(FeesBelowMin));
    }
    Ok(())
}

fn check_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let addr: ShelleyAddress =
            get_shelley_address(&output.address).ok_or(ShelleyMA(AddressDecoding))?;
        if addr.network().value() != *network_id {
            return Err(ShelleyMA(WrongNetworkID));
        }
    }
    Ok(())
}

fn check_metadata(tx_body: &TransactionBody, mtx: &MintedTx) -> ValidationResult {
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
                Err(ShelleyMA(MetadataHash))
            }
        }
        (None, None) => Ok(()),
        _ => Err(ShelleyMA(MetadataHash)),
    }
}

fn check_witnesses(
    tx_body: &TransactionBody,
    tx_wits: &MintedWitnessSet,
    utxos: &UTxOs,
) -> ValidationResult {
    let vk_wits: &mut Vec<(bool, VKeyWitness)> =
        &mut mk_alonzo_vk_wits_check_list(&tx_wits.vkeywitness, ShelleyMA(MissingVKWitness))?;
    let tx_hash: &Vec<u8> = &Vec::from(tx_body.compute_hash().as_ref());
    for input in tx_body.inputs.iter() {
        match utxos.get(&MultiEraInput::from_alonzo_compatible(input)) {
            Some(multi_era_output) => {
                if let Some(alonzo_comp_output) = MultiEraOutput::as_alonzo(multi_era_output) {
                    match get_payment_part(&alonzo_comp_output.address)
                        .ok_or(ShelleyMA(AddressDecoding))?
                    {
                        ShelleyPaymentPart::Key(payment_key_hash) => {
                            check_vk_wit(&payment_key_hash, tx_hash, vk_wits)?
                        }
                        ShelleyPaymentPart::Script(script_hash) => check_native_script_witness(
                            &script_hash,
                            &tx_wits
                                .native_script
                                .as_ref()
                                .map(|x| x.iter().map(|y| y.deref().clone()).collect()),
                        )?,
                    }
                }
            }
            None => return Err(ShelleyMA(InputNotInUTxO)),
        }
    }
    check_remaining_vk_wits(vk_wits, tx_hash)
}

fn check_vk_wit(
    payment_key_hash: &PaymentKeyHash,
    data_to_verify: &[u8],
    wits: &mut [(bool, VKeyWitness)],
) -> ValidationResult {
    for (found, vkey_wit) in wits {
        if pallas_crypto::hash::Hasher::<224>::hash(&vkey_wit.vkey.clone()) == *payment_key_hash {
            if verify_signature(vkey_wit, data_to_verify) {
                *found = true;
                return Ok(());
            } else {
                return Err(ShelleyMA(WrongSignature));
            }
        }
    }
    Err(ShelleyMA(MissingVKWitness))
}

fn check_native_script_witness(
    script_hash: &ScriptHash,
    wits: &Option<Vec<NativeScript>>,
) -> ValidationResult {
    match wits {
        Some(scripts) => {
            let mut payload: Vec<u8> = vec![0u8];
            for script in scripts.iter() {
                let _ = encode(script, &mut payload);
                if pallas_crypto::hash::Hasher::<224>::hash(&payload) == *script_hash {
                    return Ok(());
                }
            }
            Err(ShelleyMA(MissingScriptWitness))
        }
        None => Err(ShelleyMA(MissingScriptWitness)),
    }
}

fn check_remaining_vk_wits(
    wits: &mut Vec<(bool, VKeyWitness)>,
    data_to_verify: &[u8],
) -> ValidationResult {
    for (covered, vkey_wit) in wits {
        if !*covered {
            if verify_signature(vkey_wit, data_to_verify) {
                return Ok(());
            } else {
                return Err(ShelleyMA(WrongSignature));
            }
        }
    }
    Ok(())
}

fn check_minting(tx_body: &TransactionBody, mtx: &MintedTx) -> ValidationResult {
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
            for (policy, _) in minted_value.iter() {
                if native_script_wits
                    .iter()
                    .all(|script| compute_script_hash(script) != *policy)
                {
                    return Err(ShelleyMA(MintingLacksPolicy));
                }
            }
            Ok(())
        }
        None => Ok(()),
    }
}

fn compute_script_hash(script: &NativeScript) -> PolicyId {
    let mut payload = Vec::new();
    let _ = encode(script, &mut payload);
    payload.insert(0, 0);
    pallas_crypto::hash::Hasher::<224>::hash(&payload)
}
