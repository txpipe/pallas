//! Generic value operations to avoid code duplication between eras

use pallas_primitives::{
    alonzo::{Multiasset, Value},
    conway::{Multiasset as ConwayMultiasset, Value as ConwayValue},
    AssetName, Coin, PolicyId, PositiveCoin,
};
use std::collections::{BTreeMap, HashMap};

use crate::utils::ValidationError;

/// Trait for abstracting value operations across different eras
pub trait ValueOps {
    type CoinType: Copy + Clone;
    type MultiassetType: Clone;

    fn get_lovelace(&self) -> Coin;
    fn is_coin_only(&self) -> bool;
    fn make_coin(amount: Coin) -> Self;
    fn make_multiasset(coin: Coin, multiasset: Self::MultiassetType) -> Self;
    fn get_multiasset(&self) -> Option<&Self::MultiassetType>;
}

/// Implementation for Alonzo Value
impl ValueOps for Value {
    type CoinType = Coin;
    type MultiassetType = Multiasset<Coin>;

    fn get_lovelace(&self) -> Coin {
        match self {
            Value::Coin(amount) => *amount,
            Value::Multiasset(amount, _) => *amount,
        }
    }

    fn is_coin_only(&self) -> bool {
        matches!(self, Value::Coin(_))
    }

    fn make_coin(amount: Coin) -> Self {
        Value::Coin(amount)
    }

    fn make_multiasset(coin: Coin, multiasset: Self::MultiassetType) -> Self {
        Value::Multiasset(coin, multiasset)
    }

    fn get_multiasset(&self) -> Option<&Self::MultiassetType> {
        match self {
            Value::Coin(_) => None,
            Value::Multiasset(_, multiasset) => Some(multiasset),
        }
    }
}

/// Implementation for Conway Value
impl ValueOps for ConwayValue {
    type CoinType = PositiveCoin;
    type MultiassetType = ConwayMultiasset<PositiveCoin>;

    fn get_lovelace(&self) -> Coin {
        match self {
            ConwayValue::Coin(amount) => *amount,
            ConwayValue::Multiasset(amount, _) => *amount,
        }
    }

    fn is_coin_only(&self) -> bool {
        matches!(self, ConwayValue::Coin(_))
    }

    fn make_coin(amount: Coin) -> Self {
        ConwayValue::Coin(amount)
    }

    fn make_multiasset(coin: Coin, multiasset: Self::MultiassetType) -> Self {
        ConwayValue::Multiasset(coin, multiasset)
    }

    fn get_multiasset(&self) -> Option<&Self::MultiassetType> {
        match self {
            ConwayValue::Coin(_) => None,
            ConwayValue::Multiasset(_, multiasset) => Some(multiasset),
        }
    }
}

/// Trait for abstracting multiasset operations
pub trait MultiassetOps {
    type CoinType: Copy + Clone;

    fn is_empty(&self) -> bool;
    fn find_policy(&self, policy_id: &PolicyId) -> Option<BTreeMap<AssetName, Self::CoinType>>;
    fn iter(&self) -> impl Iterator<Item = (&PolicyId, &BTreeMap<AssetName, Self::CoinType>)>;
}

/// Implementation for Alonzo Multiasset
impl MultiassetOps for Multiasset<Coin> {
    type CoinType = Coin;

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn find_policy(&self, policy_id: &PolicyId) -> Option<BTreeMap<AssetName, Self::CoinType>> {
        for (policy, assets) in self.iter() {
            if policy == policy_id {
                return Some(assets.clone());
            }
        }
        None
    }

    fn iter(&self) -> impl Iterator<Item = (&PolicyId, &BTreeMap<AssetName, Self::CoinType>)> {
        self.iter()
    }
}

/// Implementation for Conway Multiasset
impl MultiassetOps for ConwayMultiasset<PositiveCoin> {
    type CoinType = PositiveCoin;

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn find_policy(&self, policy_id: &PolicyId) -> Option<BTreeMap<AssetName, Self::CoinType>> {
        for (policy, assets) in self.iter() {
            if policy == policy_id {
                return Some(assets.clone());
            }
        }
        None
    }

    fn iter(&self) -> impl Iterator<Item = (&PolicyId, &BTreeMap<AssetName, Self::CoinType>)> {
        self.iter()
    }
}

/// Generic function to find assets in a BTreeMap
pub fn find_assets_generic<T: Copy>(
    assets: &BTreeMap<AssetName, T>,
    asset_name: &AssetName,
) -> Option<T> {
    assets.get(asset_name).copied()
}

/// Generic function to add same policy assets
pub fn add_same_policy_assets_generic<T>(
    old_assets: &HashMap<AssetName, T>,
    new_assets: &BTreeMap<AssetName, T>,
) -> HashMap<AssetName, T>
where
    T: Copy + std::ops::Add<Output = T>,
{
    let mut res: HashMap<AssetName, T> = old_assets.clone();
    for (asset_name, new_amount) in new_assets.iter() {
        match res.get(asset_name) {
            Some(old_amount) => res.insert(asset_name.clone(), *old_amount + *new_amount),
            None => res.insert(asset_name.clone(), *new_amount),
        };
    }
    res
}

/// Generic function to wrap multiasset from HashMap to BTreeMap structure
pub fn wrap_multiasset_generic<T>(
    input: HashMap<PolicyId, HashMap<AssetName, T>>
) -> BTreeMap<PolicyId, BTreeMap<AssetName, T>>
where
    T: Clone
{
    input
        .into_iter()
        .map(|(policy, assets)| (policy, assets.into_iter().collect()))
        .collect()
}

/// Generic function to add two multiassets together
pub fn add_multiasset_values_generic<T, M>(
    first: &M,
    second: &M,
) -> BTreeMap<PolicyId, BTreeMap<AssetName, T>>
where
    T: Copy + std::ops::Add<Output = T> + Default,
    M: MultiassetOps<CoinType = T>,
{
    let mut res: HashMap<PolicyId, HashMap<AssetName, T>> = HashMap::new();
    
    // Add assets from first multiasset
    for (policy, assets) in first.iter() {
        let assets_map: HashMap<AssetName, T> = assets.iter().map(|(k, v)| (k.clone(), *v)).collect();
        res.insert(*policy, assets_map);
    }
    
    // Add assets from second multiasset
    for (policy, new_assets) in second.iter() {
        match res.get(policy) {
            Some(old_assets) => {
                let combined = add_same_policy_assets_generic(old_assets, new_assets);
                res.insert(*policy, combined);
            },
            None => {
                let assets_map: HashMap<AssetName, T> = new_assets.iter().map(|(k, v)| (k.clone(), *v)).collect();
                res.insert(*policy, assets_map);
            },
        }
    }
    
    wrap_multiasset_generic(res)
}

/// Generic function to coerce multiasset types
pub fn coerce_multiasset_generic<TFrom, TTo, MFrom, MTo>(
    value: &MFrom,
    converter: impl Fn(&TFrom) -> TTo,
) -> MTo
where
    MFrom: MultiassetOps<CoinType = TFrom>,
    MTo: FromIterator<(PolicyId, BTreeMap<AssetName, TTo>)>,
    TFrom: Copy,
{
    value
        .iter()
        .map(|(policy, assets)| {
            let converted_assets: BTreeMap<AssetName, TTo> = assets
                .iter()
                .map(|(asset_name, amount)| (asset_name.clone(), converter(amount)))
                .collect();
            (*policy, converted_assets)
        })
        .collect()
}

/// Generic function to check if two values are equal
pub fn values_are_equal_generic<V>(first: &V, second: &V) -> bool
where
    V: ValueOps,
    V::MultiassetType: PartialEq,
{
    if first.get_lovelace() != second.get_lovelace() {
        return false;
    }

    match (first.get_multiasset(), second.get_multiasset()) {
        (None, None) => true,
        (Some(fma), Some(sma)) => fma == sma,
        _ => false,
    }
}

/// Generic function to calculate lovelace difference or fail
pub fn lovelace_diff_or_fail_generic<V>(
    first: &V,
    second: &V,
    err: &ValidationError,
) -> Result<u64, ValidationError>
where
    V: ValueOps,
    V::MultiassetType: PartialEq + MultiassetOps,
{
    let first_lovelace = first.get_lovelace();
    let second_lovelace = second.get_lovelace();

    if first_lovelace < second_lovelace {
        return Err(err.clone());
    }

    match (first.get_multiasset(), second.get_multiasset()) {
        (None, None) => Ok(first_lovelace - second_lovelace),
        (Some(fma), None) => {
            if fma.is_empty() {
                Ok(first_lovelace - second_lovelace)
            } else {
                Err(err.clone())
            }
        }
        (None, Some(_)) => Err(err.clone()),
        (Some(fma), Some(sma)) => {
            if fma == sma {
                Ok(first_lovelace - second_lovelace)
            } else {
                Err(err.clone())
            }
        }
    }
}