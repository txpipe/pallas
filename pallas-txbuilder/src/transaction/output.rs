use indexmap::IndexMap;
use pallas_codec::utils::Bytes;
use pallas_primitives::babbage::{
    PolicyId, PseudoPostAlonzoTransactionOutput, TransactionOutput, Value,
};

#[derive(Debug, Clone)]
pub enum OutputError {
    InvalidAssetName(String),
}

#[derive(Debug, Clone, Default)]
pub struct MultiAsset {
    lovelace_amount: u64,
    assets: IndexMap<PolicyId, Vec<(Bytes, u64)>>,
}

impl MultiAsset {
    pub fn new(lovelace_amount: u64) -> Self {
        Self {
            lovelace_amount,
            ..Default::default()
        }
    }

    pub fn add(
        mut self,
        policy_id: PolicyId,
        name: impl Into<String> + Clone,
        amount: u64,
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

    fn build(self) -> Value {
        let assets = self
            .assets
            .into_iter()
            .map(|(policy_id, pair)| (policy_id, pair.into()))
            .collect::<Vec<_>>();

        Value::Multiasset(self.lovelace_amount, assets.into())
    }
}

#[derive(Debug, Clone)]
pub enum Output {
    Lovelaces { address: Bytes, value: u64 },
    MultiAsset { address: Bytes, assets: MultiAsset },
}

impl Output {
    pub fn lovelaces(address: impl Into<Bytes>, value: u64) -> Self {
        Self::Lovelaces {
            address: address.into(),
            value,
        }
    }

    pub fn multiasset(address: impl Into<Bytes>, assets: MultiAsset) -> Self {
        Self::MultiAsset {
            address: address.into(),
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
            Self::MultiAsset { address, assets } => {
                TransactionOutput::PostAlonzo(PseudoPostAlonzoTransactionOutput {
                    address,
                    value: assets.build(),
                    datum_option: None,
                    script_ref: None,
                })
            }
        }
    }
}
