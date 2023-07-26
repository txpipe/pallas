use pallas_codec::utils::Bytes;
use pallas_primitives::babbage::{PseudoPostAlonzoTransactionOutput, TransactionOutput, Value};

use crate::asset::MultiAsset;

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
