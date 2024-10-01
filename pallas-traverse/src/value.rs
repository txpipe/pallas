use std::ops::Deref;

use pallas_primitives::{alonzo, conway};

use crate::{MultiEraPolicyAssets, MultiEraValue};

impl MultiEraValue<'_> {
    pub fn into_alonzo(&self) -> alonzo::Value {
        match self {
            Self::Byron(x) => alonzo::Value::Coin(*x),
            Self::AlonzoCompatible(x) => x.deref().clone(),
            Self::Conway(x) => match x.deref() {
                conway::Value::Coin(x) => alonzo::Value::Coin(*x),
                conway::Value::Multiasset(x, assets) => {
                    let coin = *x;
                    let assets = assets
                        .iter()
                        .map(|(k, v)| {
                            let v = v.iter().map(|(k, v)| (k.clone(), v.into())).collect();
                            (*k, v)
                        })
                        .collect();

                    alonzo::Value::Multiasset(coin, assets)
                }
            },
        }
    }

    /// The amount of ADA asset expressed in Lovelace unit
    ///
    /// The value returned provides the amount of the ADA in a particular
    /// output. The value is expressed in 'lovelace' (1 ADA = 1,000,000
    /// lovelace).
    pub fn coin(&self) -> u64 {
        match self {
            Self::Byron(x) => *x,
            Self::AlonzoCompatible(x) => match x.deref() {
                alonzo::Value::Coin(c) => *c,
                alonzo::Value::Multiasset(c, _) => *c,
            },
            Self::Conway(x) => match x.deref() {
                conway::Value::Coin(c) => *c,
                conway::Value::Multiasset(c, _) => *c,
            },
        }
    }

    /// List of native assets in the output
    ///
    /// Returns a list of Asset structs where each one represent a native asset
    /// present in the output of the tx. ADA assets are not included in this
    /// list.
    pub fn assets(&self) -> Vec<MultiEraPolicyAssets<'_>> {
        match self {
            Self::Byron(_) => vec![],
            Self::AlonzoCompatible(x) => match x.deref() {
                alonzo::Value::Coin(_) => vec![],
                alonzo::Value::Multiasset(_, x) => x
                    .iter()
                    .map(|(k, v)| MultiEraPolicyAssets::AlonzoCompatibleOutput(k, v))
                    .collect(),
            },
            Self::Conway(x) => match x.deref() {
                conway::Value::Coin(_) => vec![],
                conway::Value::Multiasset(_, x) => x
                    .iter()
                    .map(|(k, v)| MultiEraPolicyAssets::ConwayOutput(k, v))
                    .collect(),
            },
        }
    }
}
