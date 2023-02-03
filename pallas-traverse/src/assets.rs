use std::ops::Deref;

use pallas_codec::utils::{Bytes, KeyValuePairs};
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, babbage};

use crate::{Asset, MultiEraOutput};

fn iter_policy_assets<'b>(
    policy: &'b Hash<28>,
    assets: &'b KeyValuePairs<Bytes, u64>,
) -> impl Iterator<Item = Asset> + 'b {
    assets
        .iter()
        .map(|(name, amount)| Asset::NativeAsset(*policy, Vec::<u8>::clone(name), *amount))
}

fn collect_multiassets(multiassets: &alonzo::Multiasset<alonzo::Coin>) -> Vec<Asset> {
    multiassets
        .iter()
        .flat_map(|(p, a)| iter_policy_assets(p, a))
        .collect::<Vec<_>>()
}

impl Asset {
    pub fn subject(&self) -> String {
        match self {
            Self::Ada(_) => String::from("ada"),
            Self::NativeAsset(p, n, _) => format!("{p}.{}", hex::encode(n)),
        }
    }

    pub fn ascii_name(&self) -> Option<String> {
        match self {
            Self::Ada(_) => None,
            Self::NativeAsset(_, n, _) => String::from_utf8(n.clone()).ok(),
        }
    }

    pub fn policy_hex(&self) -> Option<String> {
        match self {
            Asset::Ada(_) => None,
            Asset::NativeAsset(p, _, _) => Some(p.to_string()),
        }
    }
}

impl<'b> MultiEraOutput<'b> {
    /// The amount of ADA asset expressed in Lovelace unit
    ///
    /// The value returned provides the amount of the ADA in a particular
    /// output. The value is expressed in 'lovelace' (1 ADA = 1,000,000
    /// lovelace).
    pub fn lovelace_amount(&self) -> u64 {
        match self {
            MultiEraOutput::Byron(x) => x.amount,
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(x) => match x.amount {
                    babbage::Value::Coin(c) => c,
                    babbage::Value::Multiasset(c, _) => c,
                },
                babbage::MintedTransactionOutput::PostAlonzo(x) => match x.value {
                    babbage::Value::Coin(c) => c,
                    babbage::Value::Multiasset(c, _) => c,
                },
            },
            MultiEraOutput::AlonzoCompatible(x) => match x.amount {
                alonzo::Value::Coin(c) => c,
                alonzo::Value::Multiasset(c, _) => c,
            },
        }
    }

    /// List of native assets in the output
    ///
    /// Returns a list of Asset structs where each one represent a native asset
    /// present in the output of the tx. ADA assets are not included in this
    /// list.
    pub fn non_ada_assets(&self) -> Vec<Asset> {
        match self {
            MultiEraOutput::Byron(_) => vec![],
            MultiEraOutput::Babbage(x) => match x.deref().deref() {
                babbage::MintedTransactionOutput::Legacy(x) => match &x.amount {
                    babbage::Value::Coin(_) => vec![],
                    babbage::Value::Multiasset(_, x) => collect_multiassets(x),
                },
                babbage::MintedTransactionOutput::PostAlonzo(x) => match &x.value {
                    babbage::Value::Coin(_) => vec![],
                    babbage::Value::Multiasset(_, x) => collect_multiassets(x),
                },
            },
            MultiEraOutput::AlonzoCompatible(x) => match &x.amount {
                alonzo::Value::Coin(_) => vec![],
                alonzo::Value::Multiasset(_, x) => collect_multiassets(x),
            },
        }
    }

    /// List of all assets in the output
    ///
    /// Returns a list of Asset structs where each one represent either ADA or a
    /// native asset present in the output of the tx.
    pub fn assets(&self) -> Vec<Asset> {
        [
            vec![Asset::Ada(self.lovelace_amount())],
            self.non_ada_assets(),
        ]
        .concat()
    }
}
