use std::marker::PhantomData;

use indexmap::IndexMap;
use pallas_codec::utils::{Bytes, KeyValuePairs};
use pallas_primitives::babbage::PolicyId;

#[derive(Debug, Clone)]
pub enum AssetError {
    InvalidAssetName(String),
}

#[derive(Debug, Clone, Default)]
pub struct MultiAsset<T> {
    assets: IndexMap<PolicyId, Vec<(Bytes, T)>>,
    _marker: PhantomData<T>,
}

impl<T: Default + Copy> MultiAsset<T>
where
    KeyValuePairs<Bytes, T>: From<Vec<(Bytes, T)>>,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(
        mut self,
        policy_id: PolicyId,
        name: impl Into<String> + Clone,
        amount: T,
    ) -> Result<Self, AssetError> {
        let name: Bytes = hex::encode(name.clone().into())
            .try_into()
            .map_err(|_| AssetError::InvalidAssetName(name.into()))?;

        self.assets
            .entry(policy_id)
            .and_modify(|v| v.push((name.clone(), amount)))
            .or_insert(vec![(name, amount)]);

        Ok(self)
    }

    pub(crate) fn build(self) -> pallas_primitives::babbage::Multiasset<T> {
        let assets = self
            .assets
            .into_iter()
            .map(|(policy_id, pair)| (policy_id, pair.into()))
            .collect::<Vec<_>>();

        assets.into()
    }
}
