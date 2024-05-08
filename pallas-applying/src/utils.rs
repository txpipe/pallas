//! Base types used for validating transactions in each era.

pub mod environment;
pub mod validation;

pub use environment::*;
use pallas_addresses::{Address, ShelleyAddress, ShelleyPaymentPart};
use pallas_codec::{
    minicbor::encode,
    utils::{Bytes, KeepRaw, KeyValuePairs},
};
use pallas_crypto::key::ed25519::{PublicKey, Signature};
use pallas_primitives::{
    alonzo::{
        AssetName, AuxiliaryData, Coin, MintedTx as AlonzoMintedTx, Multiasset, NativeScript,
        NetworkId, PlutusScript, PolicyId, TransactionBody, VKeyWitness, Value,
    },
    babbage::{MintedTransactionBody, MintedTx as BabbageMintedTx, PlutusV2Script},
};
use pallas_traverse::{MultiEraInput, MultiEraOutput};
use std::collections::HashMap;
use std::ops::Deref;
pub use validation::*;

pub type UTxOs<'b> = HashMap<MultiEraInput<'b>, MultiEraOutput<'b>>;

pub fn get_alonzo_comp_tx_size(tx_body: &TransactionBody) -> Option<u32> {
    let mut buff: Vec<u8> = Vec::new();
    match encode(tx_body, &mut buff) {
        Ok(()) => Some(buff.len() as u32),
        Err(_) => None,
    }
}

pub fn get_babbage_tx_size(tx_body: &MintedTransactionBody) -> Option<u32> {
    let mut buff: Vec<u8> = Vec::new();
    match encode(tx_body, &mut buff) {
        Ok(()) => Some(buff.len() as u32),
        Err(_) => None,
    }
}

pub fn empty_value() -> Value {
    Value::Multiasset(0, Multiasset::<Coin>::from(Vec::new()))
}

pub fn add_values(
    first: &Value,
    second: &Value,
    err: &ValidationError,
) -> Result<Value, ValidationError> {
    match (first, second) {
        (Value::Coin(f), Value::Coin(s)) => Ok(Value::Coin(f + s)),
        (Value::Multiasset(f, fma), Value::Coin(s)) => Ok(Value::Multiasset(f + s, fma.clone())),
        (Value::Coin(f), Value::Multiasset(s, sma)) => Ok(Value::Multiasset(f + s, sma.clone())),
        (Value::Multiasset(f, fma), Value::Multiasset(s, sma)) => Ok(Value::Multiasset(
            f + s,
            coerce_to_coin(
                &add_multiasset_values(&coerce_to_i64(fma), &coerce_to_i64(sma)),
                err,
            )?,
        )),
    }
}

pub fn lovelace_diff_or_fail(
    first: &Value,
    second: &Value,
    err: &ValidationError,
) -> Result<u64, ValidationError> {
    match (first, second) {
        (Value::Coin(f), Value::Coin(s)) => {
            if f >= s {
                Ok(f - s)
            } else {
                Err(err.clone())
            }
        }
        (Value::Coin(_), Value::Multiasset(_, _)) => Err(err.clone()),
        (Value::Multiasset(f, fma), Value::Coin(s)) => {
            if f >= s && fma.is_empty() {
                Ok(f - s)
            } else {
                Err(err.clone())
            }
        }
        (Value::Multiasset(f, fma), Value::Multiasset(s, sma)) => {
            if f >= s && multi_assets_are_equal(fma, sma) {
                Ok(f - s)
            } else {
                Err(err.clone())
            }
        }
    }
}

pub fn multi_assets_are_equal(fma: &Multiasset<Coin>, sma: &Multiasset<Coin>) -> bool {
    for (fpolicy, fassets) in fma.iter() {
        match find_policy(sma, fpolicy) {
            Some(sassets) => {
                for (fasset_name, famount) in fassets.iter() {
                    // Discard the case where there is 0 of an asset
                    if *famount != 0 {
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
            }
            None => return false,
        }
    }
    true
}

pub fn add_minted_value(
    base_value: &Value,
    minted_value: &Multiasset<i64>,
    err: &ValidationError,
) -> Result<Value, ValidationError> {
    match base_value {
        Value::Coin(n) => Ok(Value::Multiasset(*n, coerce_to_coin(minted_value, err)?)),
        Value::Multiasset(n, mary_base_value) => Ok(Value::Multiasset(
            *n,
            coerce_to_coin(
                &add_multiasset_values(&coerce_to_i64(mary_base_value), minted_value),
                err,
            )?,
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

fn coerce_to_coin(
    value: &Multiasset<i64>,
    err: &ValidationError,
) -> Result<Multiasset<Coin>, ValidationError> {
    let mut res: Vec<(PolicyId, KeyValuePairs<AssetName, Coin>)> = Vec::new();
    for (policy, assets) in value.clone().to_vec().iter() {
        let mut aa: Vec<(AssetName, Coin)> = Vec::new();
        for (asset_name, amount) in assets.clone().to_vec().iter() {
            if *amount < 0 {
                return Err(err.clone());
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

pub fn values_are_equal(first: &Value, second: &Value) -> bool {
    match (first, second) {
        (Value::Coin(f), Value::Coin(s)) => f == s,
        (Value::Multiasset(..), Value::Coin(..)) => false,
        (Value::Coin(..), Value::Multiasset(..)) => false,
        (Value::Multiasset(f, fma), Value::Multiasset(s, sma)) => {
            if f != s {
                false
            } else {
                multi_assets_are_equal(fma, sma)
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

pub fn get_lovelace_from_alonzo_val(val: &Value) -> Coin {
    match val {
        Value::Coin(res) => *res,
        Value::Multiasset(res, _) => *res,
    }
}

pub fn get_network_id_value(network_id: NetworkId) -> u8 {
    match network_id {
        NetworkId::One => 0,
        NetworkId::Two => 1,
    }
}

pub fn mk_alonzo_vk_wits_check_list(
    wits: &Option<Vec<VKeyWitness>>,
    err: ValidationError,
) -> Result<Vec<(bool, VKeyWitness)>, ValidationError> {
    Ok(wits
        .clone()
        .ok_or(err)?
        .iter()
        .map(|x| (false, x.clone()))
        .collect::<Vec<(bool, VKeyWitness)>>())
}

pub fn verify_signature(vk_wit: &VKeyWitness, data_to_verify: &[u8]) -> bool {
    let mut public_key_source: [u8; PublicKey::SIZE] = [0; PublicKey::SIZE];
    public_key_source.copy_from_slice(vk_wit.vkey.as_slice());
    let public_key: PublicKey = From::<[u8; PublicKey::SIZE]>::from(public_key_source);
    let mut signature_source: [u8; Signature::SIZE] = [0; Signature::SIZE];
    signature_source.copy_from_slice(vk_wit.signature.as_slice());
    let sig: Signature = From::<[u8; Signature::SIZE]>::from(signature_source);
    public_key.verify(data_to_verify, &sig)
}

pub fn get_payment_part(address: &Bytes) -> Option<ShelleyPaymentPart> {
    let addr: ShelleyAddress = get_shelley_address(Bytes::deref(address))?;
    Some(addr.payment().clone())
}

pub fn get_shelley_address(address: &[u8]) -> Option<ShelleyAddress> {
    match Address::from_bytes(address) {
        Ok(Address::Shelley(sa)) => Some(sa),
        _ => None,
    }
}

pub fn is_byron_address(address: &[u8]) -> bool {
    matches!(Address::from_bytes(address), Ok(Address::Byron(_)))
}

pub fn aux_data_from_alonzo_minted_tx<'a>(mtx: &'a AlonzoMintedTx) -> Option<&'a [u8]> {
    Option::<KeepRaw<AuxiliaryData>>::from((mtx.auxiliary_data).clone())
        .as_ref()
        .map(KeepRaw::raw_cbor)
}

pub fn aux_data_from_babbage_minted_tx<'a>(mtx: &'a BabbageMintedTx) -> Option<&'a [u8]> {
    Option::<KeepRaw<AuxiliaryData>>::from((mtx.auxiliary_data).clone())
        .as_ref()
        .map(KeepRaw::raw_cbor)
}

pub fn get_val_size_in_words(val: &Value) -> u64 {
    let mut tx_buf: Vec<u8> = Vec::new();
    let _ = encode(val, &mut tx_buf);
    (tx_buf.len() as u64 + 7) / 8 // ceiling of the result of dividing
}

pub fn compute_native_script_hash(script: &NativeScript) -> PolicyId {
    let mut payload = Vec::new();
    let _ = encode(script, &mut payload);
    payload.insert(0, 0);
    pallas_crypto::hash::Hasher::<224>::hash(&payload)
}

pub fn compute_plutus_script_hash(script: &PlutusScript) -> PolicyId {
    let mut payload: Vec<u8> = Vec::from(script.as_ref());
    payload.insert(0, 1);
    pallas_crypto::hash::Hasher::<224>::hash(&payload)
}

pub fn compute_plutus_v2_script_hash(script: &PlutusV2Script) -> PolicyId {
    let mut payload: Vec<u8> = Vec::from(script.as_ref());
    payload.insert(0, 2);
    pallas_crypto::hash::Hasher::<224>::hash(&payload)
}
