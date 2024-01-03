//! Utilities required for Shelley-era transaction validation.

use crate::types::{
    FeePolicy,
    ShelleyMAError::*,
    ShelleyProtParams, UTxOs,
    ValidationError::{self, *},
    ValidationResult,
};
use pallas_addresses::{Address, PaymentKeyHash, ScriptHash, ShelleyAddress, ShelleyPaymentPart};
use pallas_codec::{
    minicbor::encode,
    utils::{Bytes, KeepRaw, KeyValuePairs},
};
use pallas_crypto::key::ed25519::{PublicKey, Signature};
use pallas_primitives::{
    alonzo::{
        AssetName, AuxiliaryData, Coin, MintedTx, MintedWitnessSet, Multiasset, NativeScript,
        PolicyId, TransactionBody, TransactionOutput, VKeyWitness, Value,
    },
    byron::TxOut,
};
use pallas_traverse::{ComputeHash, Era, MultiEraInput, MultiEraOutput};
use std::{collections::HashMap, ops::Deref};

// TODO: implement each of the validation rules.
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
    let size: &u64 = &get_tx_size(tx_body)?;
    let auxiliary_data_hash: &Option<Bytes> = &tx_body.auxiliary_data_hash;
    let auxiliary_data: &Option<&[u8]> = &extract_auxiliary_data(mtx);
    let minted_value: &Option<Multiasset<i64>> = &tx_body.mint;
    let native_script_wits: &Option<Vec<NativeScript>> = &mtx
        .transaction_witness_set
        .native_script
        .as_ref()
        .map(|x| x.iter().map(|y| y.deref().clone()).collect());
    check_ins_not_empty(tx_body)?;
    check_ins_in_utxos(tx_body, utxos)?;
    check_ttl(tx_body, block_slot)?;
    check_size(size, prot_pps)?;
    check_min_lovelace(tx_body, prot_pps, era)?;
    check_preservation_of_value(tx_body, utxos, era)?;
    check_fees(tx_body, size, &prot_pps.fee_policy)?;
    check_network_id(tx_body, network_id)?;
    check_metadata(auxiliary_data_hash, auxiliary_data)?;
    check_witnesses(tx_body, utxos, tx_wits)?;
    check_minting(minted_value, native_script_wits)
}

fn get_tx_size(tx_body: &TransactionBody) -> Result<u64, ValidationError> {
    let mut buff: Vec<u8> = Vec::new();
    match encode(tx_body, &mut buff) {
        Ok(()) => Ok(buff.len() as u64),
        Err(_) => Err(Shelley(UnknownTxSize)),
    }
}

fn extract_auxiliary_data<'a>(mtx: &'a MintedTx) -> Option<&'a [u8]> {
    Option::<KeepRaw<AuxiliaryData>>::from((mtx.auxiliary_data).clone())
        .as_ref()
        .map(KeepRaw::raw_cbor)
}

fn check_ins_not_empty(tx_body: &TransactionBody) -> ValidationResult {
    if tx_body.inputs.is_empty() {
        return Err(Shelley(TxInsEmpty));
    }
    Ok(())
}

fn check_ins_in_utxos(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    for input in tx_body.inputs.iter() {
        if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(input))) {
            return Err(Shelley(InputNotInUTxO));
        }
    }
    Ok(())
}

fn check_ttl(tx_body: &TransactionBody, block_slot: &u64) -> ValidationResult {
    match tx_body.ttl {
        Some(ttl) => {
            if ttl < *block_slot {
                Err(Shelley(TTLExceeded))
            } else {
                Ok(())
            }
        }
        None => Err(Shelley(AlonzoCompNotShelley)),
    }
}

fn check_size(size: &u64, prot_pps: &ShelleyProtParams) -> ValidationResult {
    if *size > prot_pps.max_tx_size {
        return Err(Shelley(MaxTxSizeExceeded));
    }
    Ok(())
}

fn check_min_lovelace(
    tx_body: &TransactionBody,
    prot_pps: &ShelleyProtParams,
    era: &Era,
) -> ValidationResult {
    for TransactionOutput { amount, .. } in &tx_body.outputs {
        match (era, amount) {
            (Era::Shelley, Value::Coin(lovelace))
            | (Era::Allegra, Value::Coin(lovelace))
            | (Era::Mary, Value::Multiasset(lovelace, _)) => {
                if *lovelace < prot_pps.min_lovelace {
                    return Err(Shelley(MinLovelaceUnreached));
                }
            }
            _ => return Err(Shelley(ValueNotShelley)),
        }
    }
    Ok(())
}

fn check_preservation_of_value(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    era: &Era,
) -> ValidationResult {
    let input: Value = get_consumed(tx_body, utxos, era)?;
    let produced: Value = get_produced(tx_body, era)?;
    let output: Value = add_values(&produced, &Value::Coin(tx_body.fee))?;
    if let Some(m) = &tx_body.mint {
        add_minted_value(&output, m)?;
    }
    if !values_are_equal(&input, &output) {
        return Err(Shelley(PreservationOfValue));
    }
    Ok(())
}

fn get_consumed(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    era: &Era,
) -> Result<Value, ValidationError> {
    let mut res: Value = empty_value();
    for input in tx_body.inputs.iter() {
        let utxo_value: &MultiEraOutput = utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .ok_or(Shelley(InputNotInUTxO))?;
        match MultiEraOutput::as_alonzo(utxo_value) {
            Some(TransactionOutput { amount, .. }) => match (amount, era) {
                (Value::Coin(..), _) => res = add_values(&res, amount)?,
                (Value::Multiasset(..), Era::Shelley) => return Err(Shelley(ValueNotShelley)),
                _ => res = add_values(&res, amount)?,
            },
            None => match MultiEraOutput::as_byron(utxo_value) {
                Some(TxOut { amount, .. }) => res = add_values(&res, &Value::Coin(*amount))?,
                _ => return Err(Shelley(InputNotInUTxO)),
            },
        }
    }
    Ok(res)
}

fn get_produced(tx_body: &TransactionBody, era: &Era) -> Result<Value, ValidationError> {
    let mut res: Value = empty_value();
    for TransactionOutput { amount, .. } in tx_body.outputs.iter() {
        match (amount, era) {
            (Value::Coin(..), _) => res = add_values(&res, amount)?,
            (Value::Multiasset(..), Era::Shelley) => return Err(Shelley(WrongEraOutput)),
            _ => res = add_values(&res, amount)?,
        }
    }
    Ok(res)
}

fn empty_value() -> Value {
    Value::Multiasset(0, Multiasset::<Coin>::from(Vec::new()))
}

fn add_values(first: &Value, second: &Value) -> Result<Value, ValidationError> {
    match (first, second) {
        (Value::Coin(f), Value::Coin(s)) => Ok(Value::Coin(f + s)),
        (Value::Multiasset(f, fma), Value::Coin(s)) => Ok(Value::Multiasset(f + s, fma.clone())),
        (Value::Coin(f), Value::Multiasset(s, sma)) => Ok(Value::Multiasset(f + s, sma.clone())),
        (Value::Multiasset(f, fma), Value::Multiasset(s, sma)) => Ok(Value::Multiasset(
            f + s,
            coerce_to_coin(&add_multiasset_values(
                &coerce_to_i64(fma),
                &coerce_to_i64(sma),
            ))?,
        )),
    }
}

fn add_minted_value(
    base_value: &Value,
    minted_value: &Multiasset<i64>,
) -> Result<Value, ValidationError> {
    match base_value {
        Value::Coin(n) => Ok(Value::Multiasset(*n, coerce_to_coin(minted_value)?)),
        Value::Multiasset(n, mary_base_value) => Ok(Value::Multiasset(
            *n,
            coerce_to_coin(&add_multiasset_values(
                &coerce_to_i64(mary_base_value),
                minted_value,
            ))?,
        )),
    }
}

fn coerce_to_i64(value: &Multiasset<Coin>) -> Multiasset<i64> {
    let mut res: Vec<(PolicyId, KeyValuePairs<AssetName, i64>)> = Vec::new();
    for (policy, assets) in value.clone().to_vec().iter() {
        let mut aa: Vec<(AssetName, i64)> = Vec::new();
        for (asset_name, amount) in assets.clone().to_vec().iter() {
            aa.push((asset_name.clone(), *amount as i64));
        }
        res.push((*policy, KeyValuePairs::<AssetName, i64>::from(aa)));
    }
    KeyValuePairs::<PolicyId, KeyValuePairs<AssetName, i64>>::from(res)
}

fn coerce_to_coin(value: &Multiasset<i64>) -> Result<Multiasset<Coin>, ValidationError> {
    let mut res: Vec<(PolicyId, KeyValuePairs<AssetName, Coin>)> = Vec::new();
    for (policy, assets) in value.clone().to_vec().iter() {
        let mut aa: Vec<(AssetName, Coin)> = Vec::new();
        for (asset_name, amount) in assets.clone().to_vec().iter() {
            if *amount < 0 {
                return Err(Shelley(NegativeValue));
            }
            aa.push((asset_name.clone(), *amount as u64));
        }
        res.push((*policy, KeyValuePairs::<AssetName, Coin>::from(aa)));
    }
    Ok(KeyValuePairs::<PolicyId, KeyValuePairs<AssetName, Coin>>::from(res))
}

fn add_multiasset_values(first: &Multiasset<i64>, second: &Multiasset<i64>) -> Multiasset<i64> {
    let mut res: HashMap<PolicyId, HashMap<AssetName, i64>> = HashMap::new();
    for (policy, new_assets) in first.iter() {
        match res.get(policy) {
            Some(old_assets) => res.insert(*policy, add_same_policy_assets(old_assets, new_assets)),
            None => res.insert(*policy, add_same_policy_assets(&HashMap::new(), new_assets)),
        };
    }
    for (policy, new_assets) in second.iter() {
        match res.get(policy) {
            Some(old_assets) => res.insert(*policy, add_same_policy_assets(old_assets, new_assets)),
            None => res.insert(*policy, add_same_policy_assets(&HashMap::new(), new_assets)),
        };
    }
    wrap_multiasset(res)
}

fn add_same_policy_assets(
    old_assets: &HashMap<AssetName, i64>,
    new_assets: &KeyValuePairs<AssetName, i64>,
) -> HashMap<AssetName, i64> {
    let mut res: HashMap<AssetName, i64> = old_assets.clone();
    for (asset_name, new_amount) in new_assets.iter() {
        match res.get(asset_name) {
            Some(old_amount) => res.insert(asset_name.clone(), old_amount + *new_amount),
            None => res.insert(asset_name.clone(), *new_amount),
        };
    }
    res
}

fn wrap_multiasset(input: HashMap<PolicyId, HashMap<AssetName, i64>>) -> Multiasset<i64> {
    Multiasset::<i64>::from(
        input
            .into_iter()
            .map(|(policy, assets)| {
                (
                    policy,
                    KeyValuePairs::<AssetName, i64>::from(
                        assets.into_iter().collect::<Vec<(AssetName, i64)>>(),
                    ),
                )
            })
            .collect::<Vec<(PolicyId, KeyValuePairs<AssetName, i64>)>>(),
    )
}

fn values_are_equal(first: &Value, second: &Value) -> bool {
    match (first, second) {
        (Value::Coin(f), Value::Coin(s)) => f == s,
        (Value::Multiasset(..), Value::Coin(..)) => false,
        (Value::Coin(..), Value::Multiasset(..)) => false,
        (Value::Multiasset(f, fma), Value::Multiasset(s, sma)) => {
            if f != s {
                false
            } else {
                for (fpolicy, fassets) in fma.iter() {
                    match find_policy(sma, fpolicy) {
                        Some(sassets) => {
                            for (fasset_name, famount) in fassets.iter() {
                                match find_assets(&sassets, fasset_name) {
                                    Some(samount) => {
                                        if *famount != samount {
                                            return false;
                                        }
                                    }
                                    None => return false,
                                };
                            }
                        }
                        None => return false,
                    }
                }
                true
            }
        }
    }
}

fn find_policy(
    mary_value: &Multiasset<Coin>,
    search_policy: &PolicyId,
) -> Option<KeyValuePairs<AssetName, Coin>> {
    for (policy, assets) in mary_value.clone().to_vec().iter() {
        if policy == search_policy {
            return Some(assets.clone());
        }
    }
    None
}

fn find_assets(assets: &KeyValuePairs<AssetName, Coin>, asset_name: &AssetName) -> Option<Coin> {
    for (an, amount) in assets.clone().to_vec().iter() {
        if an == asset_name {
            return Some(*amount);
        }
    }
    None
}

fn check_fees(tx_body: &TransactionBody, size: &u64, fee_policy: &FeePolicy) -> ValidationResult {
    if tx_body.fee < fee_policy.summand + fee_policy.multiplier * size {
        return Err(Shelley(FeesBelowMin));
    }
    Ok(())
}

fn check_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let addr: ShelleyAddress = get_shelley_address(Vec::<u8>::from(output.address.clone()))?;
        if addr.network().value() != *network_id {
            return Err(Shelley(WrongNetworkID));
        }
    }
    Ok(())
}

fn get_shelley_address(address: Vec<u8>) -> Result<ShelleyAddress, ValidationError> {
    match Address::from_bytes(&address) {
        Ok(Address::Shelley(sa)) => Ok(sa),
        Ok(_) => Err(Shelley(WrongEraOutput)),
        Err(_) => Err(Shelley(AddressDecoding)),
    }
}

fn check_metadata(
    auxiliary_data_hash: &Option<Bytes>,
    auxiliary_data_cbor: &Option<&[u8]>,
) -> ValidationResult {
    match (auxiliary_data_hash, auxiliary_data_cbor) {
        (Some(metadata_hash), Some(metadata)) => {
            if metadata_hash.as_slice()
                == pallas_crypto::hash::Hasher::<256>::hash(metadata).as_ref()
            {
                Ok(())
            } else {
                Err(Shelley(MetadataHash))
            }
        }
        (None, None) => Ok(()),
        _ => Err(Shelley(MetadataHash)),
    }
}

fn check_witnesses(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    tx_wits: &MintedWitnessSet,
) -> ValidationResult {
    let wits: &mut Vec<(bool, VKeyWitness)> = &mut mk_vkwitness_check_list(&tx_wits.vkeywitness)?;
    let tx_hash: &Vec<u8> = &Vec::from(tx_body.compute_hash().as_ref());
    for input in tx_body.inputs.iter() {
        match utxos.get(&MultiEraInput::from_alonzo_compatible(input)) {
            Some(multi_era_output) => {
                if let Some(alonzo_comp_output) = MultiEraOutput::as_alonzo(multi_era_output) {
                    match get_payment_part(alonzo_comp_output)? {
                        ShelleyPaymentPart::Key(payment_key_hash) => {
                            check_verification_key_witness(&payment_key_hash, tx_hash, wits)?
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
            None => return Err(Shelley(InputNotInUTxO)),
        }
    }
    check_remaining_verification_key_witnesses(wits, tx_hash)
}

fn mk_vkwitness_check_list(
    wits: &Option<Vec<VKeyWitness>>,
) -> Result<Vec<(bool, VKeyWitness)>, ValidationError> {
    Ok(wits
        .clone()
        .ok_or(Shelley(MissingVKWitness))?
        .iter()
        .map(|x| (false, x.clone()))
        .collect::<Vec<(bool, VKeyWitness)>>())
}

fn get_payment_part(tx_out: &TransactionOutput) -> Result<ShelleyPaymentPart, ValidationError> {
    let addr: ShelleyAddress = get_shelley_address(Vec::<u8>::from(tx_out.address.clone()))?;
    Ok(addr.payment().clone())
}

fn check_verification_key_witness(
    payment_key_hash: &PaymentKeyHash,
    data_to_verify: &Vec<u8>,
    wits: &mut Vec<(bool, VKeyWitness)>,
) -> ValidationResult {
    for (found, VKeyWitness { vkey, signature }) in wits {
        if pallas_crypto::hash::Hasher::<224>::hash(vkey) == *payment_key_hash {
            let mut public_key_source: [u8; PublicKey::SIZE] = [0; PublicKey::SIZE];
            public_key_source.copy_from_slice(vkey.as_slice());
            let public_key: PublicKey = From::<[u8; PublicKey::SIZE]>::from(public_key_source);
            let mut signature_source: [u8; Signature::SIZE] = [0; Signature::SIZE];
            signature_source.copy_from_slice(signature.as_slice());
            let sig: Signature = From::<[u8; Signature::SIZE]>::from(signature_source);
            if public_key.verify(data_to_verify, &sig) {
                *found = true;
                return Ok(());
            } else {
                return Err(Shelley(WrongSignature));
            }
        }
    }
    Err(Shelley(MissingVKWitness))
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
            Err(Shelley(MissingScriptWitness))
        }
        None => Err(Shelley(MissingScriptWitness)),
    }
}

fn check_remaining_verification_key_witnesses(
    wits: &mut Vec<(bool, VKeyWitness)>,
    data_to_verify: &Vec<u8>,
) -> ValidationResult {
    for (covered, VKeyWitness { vkey, signature }) in wits {
        if !*covered {
            let mut public_key_source: [u8; PublicKey::SIZE] = [0; PublicKey::SIZE];
            public_key_source.copy_from_slice(vkey.as_slice());
            let public_key: PublicKey = From::<[u8; PublicKey::SIZE]>::from(public_key_source);
            let mut signature_source: [u8; Signature::SIZE] = [0; Signature::SIZE];
            signature_source.copy_from_slice(signature.as_slice());
            let sig: Signature = From::<[u8; Signature::SIZE]>::from(signature_source);
            if !public_key.verify(data_to_verify, &sig) {
                return Err(Shelley(WrongSignature));
            }
        }
    }
    Ok(())
}

fn check_minting(
    values: &Option<Multiasset<i64>>,
    scripts: &Option<Vec<NativeScript>>,
) -> ValidationResult {
    match (values, scripts) {
        (None, _) => Ok(()),
        (Some(_), None) => Err(Shelley(MintingLacksPolicy)),
        (Some(minted_value), Some(native_script_wits)) => {
            for (policy, _) in minted_value.iter() {
                if check_policy(policy, native_script_wits) {
                    return Ok(());
                }
            }
            Ok(())
        }
    }
}

fn check_policy(policy: &PolicyId, native_script_wits: &[NativeScript]) -> bool {
    for script in native_script_wits.iter() {
        let hashed_script: PolicyId = compute_script_hash(script);
        if *policy == hashed_script {
            return true;
        }
    }
    false
}

fn compute_script_hash(script: &NativeScript) -> PolicyId {
    let mut payload = Vec::new();
    let _ = encode(script, &mut payload);
    payload.insert(0, 0);
    pallas_crypto::hash::Hasher::<224>::hash(&payload)
}
