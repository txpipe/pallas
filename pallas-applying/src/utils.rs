//! Base types used for validating transactions in each era.

pub mod environment;
pub mod validation;

pub use environment::*;
use pallas_codec::utils::KeyValuePairs;
use pallas_primitives::alonzo::{AssetName, Coin, Multiasset, PolicyId, Value};
use pallas_traverse::{MultiEraInput, MultiEraOutput};
use std::collections::HashMap;
pub use validation::*;

pub type UTxOs<'b> = HashMap<MultiEraInput<'b>, MultiEraOutput<'b>>;

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
