//! Generic value operations to avoid code duplication between eras

use pallas_primitives::{
    alonzo::{Multiasset, Value},
    conway::{Multiasset as ConwayMultiasset, Value as ConwayValue},
    AssetName, Coin, PolicyId, PositiveCoin,
};
use std::collections::BTreeMap;

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