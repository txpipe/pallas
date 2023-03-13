use pallas_crypto::hash::Hash;
use pallas_primitives::alonzo;

use crate::MultiEraAsset;

impl<'b> MultiEraAsset<'b> {
    pub fn collect_alonzo_compatible_output(source: &'b alonzo::Multiasset<u64>) -> Vec<Self> {
        source
            .iter()
            .flat_map(|(policy, assets)| {
                assets.iter().map(|(name, amount)| {
                    MultiEraAsset::AlonzoCompatible(policy, name, *amount as i64)
                })
            })
            .collect::<Vec<_>>()
    }

    pub fn collect_alonzo_compatible_mint(source: &'b alonzo::Multiasset<i64>) -> Vec<Self> {
        source
            .iter()
            .flat_map(|(policy, assets)| {
                assets
                    .iter()
                    .map(|(name, amount)| MultiEraAsset::AlonzoCompatible(policy, name, *amount))
            })
            .collect::<Vec<_>>()
    }

    pub fn policy(&self) -> Option<&Hash<28>> {
        match self {
            Self::AlonzoCompatible(x, ..) => Some(*x),
            Self::Lovelace(_) => None,
        }
    }

    pub fn name(&self) -> Option<&[u8]> {
        match self {
            Self::AlonzoCompatible(_, n, _) => Some(n.as_ref()),
            Self::Lovelace(_) => None,
        }
    }

    pub fn coin(&self) -> i64 {
        match self {
            Self::AlonzoCompatible(_, _, x) => *x,
            Self::Lovelace(x) => *x as i64,
        }
    }

    pub fn as_alonzo(&self) -> Option<(&alonzo::PolicyId, &alonzo::AssetName, i64)> {
        match self {
            Self::AlonzoCompatible(a, b, c) => Some((*a, *b, *c)),
            _ => None,
        }
    }

    pub fn to_subject(&self) -> Option<String> {
        match self {
            Self::AlonzoCompatible(p, n, _) => Some(format!("{p}.{}", hex::encode(n.to_vec()))),
            _ => None,
        }
    }

    pub fn to_ascii_name(&self) -> Option<String> {
        match self {
            Self::AlonzoCompatible(_, n, _) => String::from_utf8(n.to_vec()).ok(),
            _ => None,
        }
    }
}
