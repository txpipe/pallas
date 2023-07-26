use std::marker::PhantomData;

use indexmap::IndexMap;
use pallas_codec::utils::{Bytes, KeyValuePairs};
use pallas_primitives::babbage::{
    PolicyId, PseudoPostAlonzoTransactionOutput, TransactionOutput, Value,
};

#[derive(Debug, Clone)]
pub enum OutputError {
    InvalidAssetName(String),
}

#[derive(Debug, Clone, Default)]
pub struct MultiAsset<T> {
    assets: IndexMap<PolicyId, Vec<(Bytes, T)>>,
    _marker: PhantomData<T>,
}

impl<T: Default + Copy> MultiAsset<T>
where
    KeyValuePairs<Bytes, u64>: From<Vec<(Bytes, T)>>,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(
        mut self,
        policy_id: PolicyId,
        name: impl Into<String> + Clone,
        amount: T,
    ) -> Result<Self, OutputError> {
        let name: Bytes = hex::encode(name.clone().into())
            .try_into()
            .map_err(|_| OutputError::InvalidAssetName(name.into()))?;

        self.assets
            .entry(policy_id)
            .and_modify(|v| v.push((name.clone(), amount)))
            .or_insert(vec![(name, amount)]);

        Ok(self)
    }

    pub(crate) fn build(
        self,
    ) -> pallas_primitives::babbage::Multiasset<pallas_primitives::babbage::Coin> {
        let assets = self
            .assets
            .into_iter()
            .map(|(policy_id, pair)| (policy_id, pair.into()))
            .collect::<Vec<_>>();

        assets.into()
    }
}

#[derive(Debug, Clone)]
pub enum Output {
    Lovelaces {
        address: Bytes,
        value: u64,
    },
    MultiAsset {
        address: Bytes,
        value: u64,
        assets: MultiAsset<u64>,
    },
}

impl Output {
    pub fn lovelaces(address: impl Into<Bytes>, value: u64) -> Self {
        Self::Lovelaces {
            address: address.into(),
            value,
        }
    }

    pub fn multiasset(address: impl Into<Bytes>, lovelaces: u64, assets: MultiAsset<u64>) -> Self {
        Self::MultiAsset {
            address: address.into(),
            value: lovelaces,
            assets,
        }
    }

    pub fn build(self) -> TransactionOutput {
        match self {
            Self::Lovelaces { address, value } => {
                TransactionOutput::PostAlonzo(PseudoPostAlonzoTransactionOutput {
                    address,
                    value: Value::Coin(value),
                    datum_option: None,
                    script_ref: None,
                })
            }
            Self::MultiAsset {
                address,
                assets,
                value,
            } => TransactionOutput::PostAlonzo(PseudoPostAlonzoTransactionOutput {
                address,
                value: Value::Multiasset(value, assets.build()),
                datum_option: None,
                script_ref: None,
            }),
        }
    }
}
