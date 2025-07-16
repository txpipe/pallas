//! Base types used for validating transactions in each era.

pub mod environment;
pub mod validation;
pub mod value_ops;

pub use environment::*;
pub use value_ops::{MultiassetOps};
use pallas_addresses::{Address, ShelleyAddress, ShelleyPaymentPart};
use pallas_codec::{
    minicbor::{encode, Encode},
    utils::{Bytes, Nullable},
};
use pallas_crypto::key::ed25519::{PublicKey, Signature};
use pallas_primitives::{
    alonzo::{Multiasset, NativeScript, Tx as AlonzoTx, VKeyWitness, Value},
    babbage::Tx as BabbageTx,
    conway::{Multiasset as ConwayMultiasset, Tx as ConwayTx, Value as ConwayValue},
    AddrKeyhash, AssetName, Coin, Epoch, GenesisDelegateHash, Genesishash, Hash, NetworkId,
    NonZeroInt, PlutusScript, PolicyId, PoolKeyhash, PoolMetadata, PositiveCoin, Relay,
    RewardAccount, StakeCredential, TransactionIndex, UnitInterval, VrfKeyhash,
};

use pallas_traverse::{time::Slot, Era, MultiEraInput, MultiEraOutput, MultiEraUpdate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Deref;
pub use validation::*;

pub type TxHash = Hash<32>;
pub type TxoIdx = u32;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct EraCbor(pub Era, pub Vec<u8>);

impl From<(Era, Vec<u8>)> for EraCbor {
    fn from(value: (Era, Vec<u8>)) -> Self {
        Self(value.0, value.1)
    }
}

impl From<EraCbor> for (Era, Vec<u8>) {
    fn from(value: EraCbor) -> Self {
        (value.0, value.1)
    }
}

impl From<MultiEraOutput<'_>> for EraCbor {
    fn from(value: MultiEraOutput<'_>) -> Self {
        EraCbor(value.era(), value.encode())
    }
}

impl<'a> TryFrom<&'a EraCbor> for MultiEraOutput<'a> {
    type Error = pallas_codec::minicbor::decode::Error;

    fn try_from(value: &'a EraCbor) -> Result<Self, Self::Error> {
        MultiEraOutput::decode(value.0, &value.1)
    }
}

impl TryFrom<EraCbor> for MultiEraUpdate<'_> {
    type Error = pallas_codec::minicbor::decode::Error;

    fn try_from(value: EraCbor) -> Result<Self, Self::Error> {
        MultiEraUpdate::decode_for_era(value.0, &value.1)
    }
}

pub type UtxoBody<'a> = MultiEraOutput<'a>;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
pub struct TxoRef(pub TxHash, pub TxoIdx);

impl From<(TxHash, TxoIdx)> for TxoRef {
    fn from(value: (TxHash, TxoIdx)) -> Self {
        Self(value.0, value.1)
    }
}

impl From<TxoRef> for (TxHash, TxoIdx) {
    fn from(value: TxoRef) -> Self {
        (value.0, value.1)
    }
}

impl From<&MultiEraInput<'_>> for TxoRef {
    fn from(value: &MultiEraInput<'_>) -> Self {
        TxoRef(*value.hash(), value.index() as u32)
    }
}

pub type UtxoMap = HashMap<TxoRef, EraCbor>;

pub type UtxoSet = HashSet<TxoRef>;

pub type UTxOs<'b> = HashMap<MultiEraInput<'b>, MultiEraOutput<'b>>;

/// Trait for calculating transaction size
pub trait TxSizeCalculator {
    fn calculate_tx_size(&self) -> Option<u32>;
}

impl<'a> TxSizeCalculator for AlonzoTx<'a> {
    fn calculate_tx_size(&self) -> Option<u32> {
        Some(match &self.auxiliary_data {
            Nullable::Some(aux_data) => {
                (aux_data.raw_cbor().len()
                    + self.transaction_body.raw_cbor().len()
                    + self.transaction_witness_set.raw_cbor().len()) as u32
            }
            _ => {
                (self.transaction_body.raw_cbor().len() + self.transaction_witness_set.raw_cbor().len())
                    as u32
            }
        })
    }
}

impl<'a> TxSizeCalculator for BabbageTx<'a> {
    fn calculate_tx_size(&self) -> Option<u32> {
        let mut buff: Vec<u8> = Vec::new();
        match encode(self, &mut buff) {
            Ok(()) => Some(buff.len() as u32),
            Err(_) => None,
        }
    }
}

impl<'a> TxSizeCalculator for ConwayTx<'a> {
    fn calculate_tx_size(&self) -> Option<u32> {
        let mut buff: Vec<u8> = Vec::new();
        match encode(self, &mut buff) {
            Ok(()) => Some(buff.len() as u32),
            Err(_) => None,
        }
    }
}

/// Generic function to calculate transaction size
pub fn get_tx_size<T: TxSizeCalculator>(tx: &T) -> Option<u32> {
    tx.calculate_tx_size()
}

// Backward compatibility functions
pub fn get_alonzo_comp_tx_size(mtx: &AlonzoTx) -> u32 {
    get_tx_size(mtx).unwrap() // Alonzo always returns Some
}

pub fn get_babbage_tx_size(mtx: &BabbageTx) -> Option<u32> {
    get_tx_size(mtx)
}

pub fn get_conway_tx_size(mtx: &ConwayTx) -> Option<u32> {
    get_tx_size(mtx)
}

pub fn empty_value() -> Value {
    Value::Multiasset(0, std::collections::BTreeMap::new())
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

pub fn conway_add_values(
    first: &ConwayValue,
    second: &ConwayValue,
    err: &ValidationError,
) -> Result<ConwayValue, ValidationError> {
    match (first, second) {
        (ConwayValue::Coin(f), ConwayValue::Coin(s)) => Ok(ConwayValue::Coin(f + s)),
        (ConwayValue::Multiasset(f, fma), ConwayValue::Coin(s)) => {
            Ok(ConwayValue::Multiasset(f + s, fma.clone()))
        }
        (ConwayValue::Coin(f), ConwayValue::Multiasset(s, sma)) => {
            Ok(ConwayValue::Multiasset(f + s, sma.clone()))
        }
        (ConwayValue::Multiasset(f, fma), ConwayValue::Multiasset(s, sma)) => {
            Ok(ConwayValue::Multiasset(
                f + s,
                conway_coerce_to_coin(
                    &conway_add_multiasset_values(&coerce_to_u64(fma), &coerce_to_u64(sma)),
                    err,
                )?,
            ))
        }
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

pub fn conway_lovelace_diff_or_fail(
    first: &ConwayValue,
    second: &ConwayValue,
    err: &ValidationError,
) -> Result<u64, ValidationError> {
    match (first, second) {
        (ConwayValue::Coin(f), ConwayValue::Coin(s)) => {
            if f >= s {
                Ok(f - s)
            } else {
                Err(err.clone())
            }
        }
        (ConwayValue::Coin(_), ConwayValue::Multiasset(_, _)) => Err(err.clone()),
        (ConwayValue::Multiasset(f, fma), ConwayValue::Coin(s)) => {
            if f >= s && fma.is_empty() {
                Ok(f - s)
            } else {
                Err(err.clone())
            }
        }
        (ConwayValue::Multiasset(f, fma), ConwayValue::Multiasset(s, sma)) => {
            if f >= s && conway_multi_assets_are_equal(fma, sma) {
                Ok(f - s)
            } else {
                Err(err.clone())
            }
        }
    }
}

pub fn multi_assets_are_equal(fma: &Multiasset<Coin>, sma: &Multiasset<Coin>) -> bool {
    multi_asset_included(fma, sma) && multi_asset_included(sma, fma)
}
pub fn conway_multi_assets_are_equal(
    fma: &ConwayMultiasset<PositiveCoin>,
    sma: &ConwayMultiasset<PositiveCoin>,
) -> bool {
    conway_multi_asset_included(fma, sma) && conway_multi_asset_included(sma, fma)
}

pub fn multi_asset_included(fma: &Multiasset<Coin>, sma: &Multiasset<Coin>) -> bool {
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

pub fn conway_multi_asset_included(
    fma: &ConwayMultiasset<PositiveCoin>,
    sma: &ConwayMultiasset<PositiveCoin>,
) -> bool {
    for (fpolicy, fassets) in fma.iter() {
        match conway_find_policy(sma, fpolicy) {
            Some(sassets) => {
                for (fasset_name, famount) in fassets.iter() {
                    // Discard the case where there is 0 of an asset
                    if *famount >= PositiveCoin::try_from(1).unwrap() {
                        match conway_find_assets(&sassets, fasset_name) {
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
pub fn conway_add_minted_value(
    base_value: &ConwayValue,
    minted_value: &ConwayMultiasset<u64>,
    err: &ValidationError,
) -> Result<ConwayValue, ValidationError> {
    match base_value {
        ConwayValue::Coin(n) => Ok(ConwayValue::Multiasset(
            *n,
            conway_coerce_to_coin(minted_value, err)?,
        )),
        ConwayValue::Multiasset(n, mary_base_value) => Ok(ConwayValue::Multiasset(
            *n,
            conway_coerce_to_coin(
                &conway_add_multiasset_values(&coerce_to_u64(mary_base_value), minted_value),
                err,
            )?,
        )),
    }
}

pub fn conway_add_minted_non_zero(
    base_value: &ConwayValue,
    minted_value: &ConwayMultiasset<NonZeroInt>,
    err: &ValidationError,
) -> Result<ConwayValue, ValidationError> {
    match base_value {
        ConwayValue::Coin(n) => Ok(ConwayValue::Multiasset(
            *n,
            conway_coerce_to_non_zero_coin(minted_value, err)?,
        )),
        ConwayValue::Multiasset(n, mary_base_value) => Ok(ConwayValue::Multiasset(
            *n,
            conway_coerce_to_coin(
                &conway_add_multiasset_non_zero_values(
                    &coerce_to_u64(mary_base_value),
                    minted_value,
                ),
                err,
            )?,
        )),
    }
}

// Generic coercion functions using functional approach to reduce duplication
fn coerce_to_i64(value: &Multiasset<Coin>) -> Multiasset<i64> {
    value
        .iter()
        .map(|(policy, assets)| {
            let converted_assets: BTreeMap<AssetName, i64> = assets
                .iter()
                .map(|(asset_name, amount)| (asset_name.clone(), *amount as i64))
                .collect();
            (*policy, converted_assets)
        })
        .collect()
}

fn coerce_to_u64(value: &ConwayMultiasset<PositiveCoin>) -> ConwayMultiasset<u64> {
    value
        .iter()
        .map(|(policy, assets)| {
            let converted_assets: BTreeMap<AssetName, u64> = assets
                .iter()
                .map(|(asset_name, amount)| (asset_name.clone(), (*amount).into()))
                .collect();
            (*policy, converted_assets)
        })
        .collect()
}

// Simplified coercion functions - keeping some duplication for type safety
fn coerce_to_coin(
    value: &Multiasset<i64>,
    _err: &ValidationError,
) -> Result<Multiasset<Coin>, ValidationError> {
    let result: Vec<(PolicyId, BTreeMap<AssetName, Coin>)> = value
        .iter()
        .map(|(policy, assets)| {
            let converted_assets: BTreeMap<AssetName, Coin> = assets
                .iter()
                .map(|(asset_name, amount)| (asset_name.clone(), *amount as u64))
                .collect();
            (*policy, converted_assets)
        })
        .collect();
    Ok(result.into_iter().collect())
}

fn conway_coerce_to_coin(
    value: &ConwayMultiasset<u64>,
    _err: &ValidationError,
) -> Result<ConwayMultiasset<PositiveCoin>, ValidationError> {
    let result: Vec<(PolicyId, BTreeMap<AssetName, PositiveCoin>)> = value
        .iter()
        .map(|(policy, assets)| {
            let converted_assets: BTreeMap<AssetName, PositiveCoin> = assets
                .iter()
                .map(|(asset_name, amount)| (asset_name.clone(), PositiveCoin::try_from(*amount).unwrap()))
                .collect();
            (*policy, converted_assets)
        })
        .collect();
    Ok(result.into_iter().collect())
}

fn conway_coerce_to_non_zero_coin(
    value: &ConwayMultiasset<NonZeroInt>,
    _err: &ValidationError,
) -> Result<ConwayMultiasset<PositiveCoin>, ValidationError> {
    let result: Vec<(PolicyId, BTreeMap<AssetName, PositiveCoin>)> = value
        .iter()
        .map(|(policy, assets)| {
            let converted_assets: BTreeMap<AssetName, PositiveCoin> = assets
                .iter()
                .map(|(asset_name, amount)| (asset_name.clone(), PositiveCoin::try_from(i64::from(amount) as u64).unwrap()))
                .collect();
            (*policy, converted_assets)
        })
        .collect();
    Ok(result.into_iter().collect())
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

fn conway_add_multiasset_values(
    first: &ConwayMultiasset<u64>,
    second: &ConwayMultiasset<u64>,
) -> ConwayMultiasset<u64> {
    let mut res: HashMap<PolicyId, HashMap<AssetName, u64>> = HashMap::new();
    for (policy, new_assets) in first.iter() {
        match res.get(policy) {
            Some(old_assets) => res.insert(
                *policy,
                conway_add_same_policy_assets(old_assets, new_assets),
            ),
            None => res.insert(
                *policy,
                conway_add_same_policy_assets(&HashMap::new(), new_assets),
            ),
        };
    }
    for (policy, new_assets) in second.iter() {
        match res.get(policy) {
            Some(old_assets) => res.insert(
                *policy,
                conway_add_same_policy_assets(old_assets, new_assets),
            ),
            None => res.insert(
                *policy,
                conway_add_same_policy_assets(&HashMap::new(), new_assets),
            ),
        };
    }
    conway_wrap_multiasset(res)
}

fn conway_add_multiasset_non_zero_values(
    first: &ConwayMultiasset<u64>,
    second: &ConwayMultiasset<NonZeroInt>,
) -> ConwayMultiasset<u64> {
    let mut res: HashMap<PolicyId, HashMap<AssetName, u64>> = HashMap::new();
    for (policy, new_assets) in first.iter() {
        match res.get(policy) {
            Some(old_assets) => res.insert(
                *policy,
                conway_add_same_policy_assets(old_assets, new_assets),
            ),
            None => res.insert(
                *policy,
                conway_add_same_policy_assets(&HashMap::new(), new_assets),
            ),
        };
    }
    for (policy, new_assets) in second.iter() {
        match res.get(policy) {
            Some(old_assets) => res.insert(
                *policy,
                conway_add_same_non_zero_policy_assets(old_assets, new_assets),
            ),
            None => res.insert(
                *policy,
                conway_add_same_non_zero_policy_assets(&HashMap::new(), new_assets),
            ),
        };
    }
    conway_wrap_multiasset(res)
}

// Generic functions using value_ops - replacing era-specific versions
fn add_same_policy_assets(
    old_assets: &HashMap<AssetName, i64>,
    new_assets: &std::collections::BTreeMap<AssetName, i64>,
) -> HashMap<AssetName, i64> {
    value_ops::add_same_policy_assets_generic(old_assets, new_assets)
}

fn conway_add_same_policy_assets(
    old_assets: &HashMap<AssetName, u64>,
    new_assets: &std::collections::BTreeMap<AssetName, u64>,
) -> HashMap<AssetName, u64> {
    value_ops::add_same_policy_assets_generic(old_assets, new_assets)
}

fn conway_add_same_non_zero_policy_assets(
    old_assets: &HashMap<AssetName, u64>,
    new_assets: &std::collections::BTreeMap<AssetName, NonZeroInt>,
) -> HashMap<AssetName, u64> {
    let mut res: HashMap<AssetName, u64> = old_assets.clone();
    for (asset_name, new_amount) in new_assets.iter() {
        match res.get(asset_name) {
            Some(old_amount) => res.insert(
                asset_name.clone(),
                old_amount + i64::from(new_amount) as u64,
            ),
            None => res.insert(asset_name.clone(), i64::from(new_amount) as u64),
        };
    }
    res
}

fn wrap_multiasset(input: HashMap<PolicyId, HashMap<AssetName, i64>>) -> Multiasset<i64> {
    value_ops::wrap_multiasset_generic(input)
}

fn conway_wrap_multiasset(
    input: HashMap<PolicyId, HashMap<AssetName, u64>>,
) -> ConwayMultiasset<u64> {
    value_ops::wrap_multiasset_generic(input)
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

pub fn conway_values_are_equal(first: &ConwayValue, second: &ConwayValue) -> bool {
    match (first, second) {
        (ConwayValue::Coin(f), ConwayValue::Coin(s)) => f == s,
        (ConwayValue::Multiasset(..), ConwayValue::Coin(..)) => false,
        (ConwayValue::Coin(..), ConwayValue::Multiasset(..)) => false,
        (ConwayValue::Multiasset(f, fma), ConwayValue::Multiasset(s, sma)) => {
            if f != s {
                false
            } else {
                conway_multi_assets_are_equal(fma, sma)
            }
        }
    }
}

// Generic functions using value_ops traits - these replace the era-specific versions
fn find_policy(
    mary_value: &Multiasset<Coin>,
    search_policy: &PolicyId,
) -> Option<std::collections::BTreeMap<AssetName, Coin>> {
    mary_value.find_policy(search_policy)
}

fn conway_find_policy(
    mary_value: &ConwayMultiasset<PositiveCoin>,
    search_policy: &PolicyId,
) -> Option<std::collections::BTreeMap<AssetName, PositiveCoin>> {
    mary_value.find_policy(search_policy)
}

fn find_assets(
    assets: &std::collections::BTreeMap<AssetName, Coin>,
    asset_name: &AssetName,
) -> Option<Coin> {
    value_ops::find_assets_generic(assets, asset_name)
}

fn conway_find_assets(
    assets: &std::collections::BTreeMap<AssetName, PositiveCoin>,
    asset_name: &AssetName,
) -> Option<PositiveCoin> {
    value_ops::find_assets_generic(assets, asset_name)
}

/// Generic function to get lovelace from any value using the ValueOps trait
pub fn get_lovelace_from_value<V: value_ops::ValueOps>(val: &V) -> Coin {
    val.get_lovelace()
}

// Backward compatibility functions
pub fn get_lovelace_from_alonzo_val(val: &Value) -> Coin {
    get_lovelace_from_value(val)
}

pub fn get_lovelace_from_conway_val(val: &ConwayValue) -> Coin {
    get_lovelace_from_value(val)
}

#[deprecated(since = "0.31.0", note = "use `u8::from(...)` instead")]
pub fn get_network_id_value(network_id: NetworkId) -> u8 {
    u8::from(network_id)
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

/// Trait for accessing auxiliary data from transactions
pub trait AuxDataAccess {
    fn get_aux_data(&self) -> Option<&[u8]>;
}

impl<'a> AuxDataAccess for AlonzoTx<'a> {
    fn get_aux_data(&self) -> Option<&[u8]> {
        self.auxiliary_data.as_ref().map(|x| x.raw_cbor()).into()
    }
}

impl<'a> AuxDataAccess for BabbageTx<'a> {
    fn get_aux_data(&self) -> Option<&[u8]> {
        self.auxiliary_data.as_ref().map(|x| x.raw_cbor()).into()
    }
}

impl<'a> AuxDataAccess for ConwayTx<'a> {
    fn get_aux_data(&self) -> Option<&[u8]> {
        self.auxiliary_data.as_ref().map(|x| x.raw_cbor()).into()
    }
}

/// Generic function to get auxiliary data from any transaction
pub fn get_aux_data_from_tx<T: AuxDataAccess>(tx: &T) -> Option<&[u8]> {
    tx.get_aux_data()
}

// Backward compatibility functions - can be removed later
pub fn aux_data_from_alonzo_tx<'a>(mtx: &'a AlonzoTx) -> Option<&'a [u8]> {
    get_aux_data_from_tx(mtx)
}

pub fn aux_data_from_babbage_tx<'a>(mtx: &'a BabbageTx) -> Option<&'a [u8]> {
    get_aux_data_from_tx(mtx)
}

pub fn aux_data_from_conway_tx<'a>(mtx: &'a ConwayTx) -> Option<&'a [u8]> {
    get_aux_data_from_tx(mtx)
}

/// Generic function to calculate value size in words
pub fn get_val_size_in_words_generic<T>(val: &T) -> u64 
where 
    T: for<'a> Encode<()>,
{
    let mut tx_buf: Vec<u8> = Vec::new();
    let _ = encode(val, &mut tx_buf);
    (tx_buf.len() as u64).div_ceil(8) // ceiling of the result of dividing
}

// Backward compatibility functions
pub fn get_val_size_in_words(val: &Value) -> u64 {
    get_val_size_in_words_generic(val)
}

pub fn conway_get_val_size_in_words(val: &ConwayValue) -> u64 {
    get_val_size_in_words_generic(val)
}

pub fn compute_native_script_hash(script: &NativeScript) -> PolicyId {
    let mut payload = Vec::new();
    let _ = encode(script, &mut payload);
    payload.insert(0, 0);
    pallas_crypto::hash::Hasher::<224>::hash(&payload)
}

#[deprecated(since = "0.31.0", note = "use `compute_plutus_v1_script_hash` instead")]
pub fn compute_plutus_script_hash(script: &PlutusScript<1>) -> PolicyId {
    compute_plutus_v1_script_hash(script)
}

pub fn compute_plutus_v1_script_hash(script: &PlutusScript<1>) -> PolicyId {
    let mut payload: Vec<u8> = Vec::from(script.as_ref());
    payload.insert(0, 1);
    pallas_crypto::hash::Hasher::<224>::hash(&payload)
}

pub fn compute_plutus_v2_script_hash(script: &PlutusScript<2>) -> PolicyId {
    let mut payload: Vec<u8> = Vec::from(script.as_ref());
    payload.insert(0, 2);
    pallas_crypto::hash::Hasher::<224>::hash(&payload)
}

pub fn compute_plutus_v3_script_hash(script: &PlutusScript<3>) -> PolicyId {
    let mut payload: Vec<u8> = Vec::from(script.as_ref());
    payload.insert(0, 3);
    pallas_crypto::hash::Hasher::<224>::hash(&payload)
}

pub type CertificateIndex = u32;

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct CertPointer {
    pub slot: Slot,
    pub tx_ix: TransactionIndex,
    pub cert_ix: CertificateIndex,
}

pub type GenesisDelegation = HashMap<Genesishash, (GenesisDelegateHash, VrfKeyhash)>;
pub type FutGenesisDelegation = HashMap<(Slot, Genesishash), (GenesisDelegateHash, VrfKeyhash)>;
pub type InstantaneousRewards = (
    HashMap<StakeCredential, Coin>,
    HashMap<StakeCredential, Coin>,
);

#[derive(Default, Clone)] // for testing
pub struct DState {
    pub rewards: HashMap<StakeCredential, Coin>,
    pub delegations: HashMap<StakeCredential, PoolKeyhash>,
    pub ptrs: HashMap<CertPointer, StakeCredential>,
    pub fut_gen_delegs: FutGenesisDelegation,
    pub gen_delegs: GenesisDelegation,
    pub inst_rewards: InstantaneousRewards,
}

// Essentially part of the `PoolRegistration` component of `Certificate` at
// alonzo/src/model.rs
#[derive(Clone, Debug)]
pub struct PoolParam {
    pub vrf_keyhash: VrfKeyhash,
    pub pledge: Coin,
    pub cost: Coin,
    pub margin: UnitInterval,
    pub reward_account: RewardAccount, // FIXME: Should be a `StakeCredential`, or `Hash<_>`???
    pub pool_owners: Vec<AddrKeyhash>,
    pub relays: Vec<Relay>,
    pub pool_metadata: Option<PoolMetadata>,
}

#[derive(Default, Clone)] // for testing
pub struct PState {
    pub pool_params: HashMap<PoolKeyhash, PoolParam>,
    pub fut_pool_params: HashMap<PoolKeyhash, PoolParam>,
    pub retiring: HashMap<PoolKeyhash, Epoch>,
}

// Originally `DPState` in ShelleyMA specs, then updated to
// `CertState` in Haskell sources at Intersect (#3369).
#[non_exhaustive]
#[derive(Default, Clone)] // for testing
pub struct CertState {
    pub pstate: PState,
    pub dstate: DState,
}
