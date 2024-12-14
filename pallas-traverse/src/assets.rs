use pallas_crypto::hash::Hash;

use crate::{MultiEraAsset, MultiEraPolicyAssets};

impl MultiEraPolicyAssets<'_> {
    pub fn policy(&self) -> &Hash<28> {
        match self {
            MultiEraPolicyAssets::AlonzoCompatibleMint(x, _) => x,
            MultiEraPolicyAssets::AlonzoCompatibleOutput(x, _) => x,
            MultiEraPolicyAssets::ConwayMint(x, _) => x,
            MultiEraPolicyAssets::ConwayOutput(x, _) => x,
        }
    }

    pub fn is_output(&self) -> bool {
        match self {
            MultiEraPolicyAssets::AlonzoCompatibleMint(_, _) => false,
            MultiEraPolicyAssets::AlonzoCompatibleOutput(_, _) => true,
            MultiEraPolicyAssets::ConwayMint(_, _) => false,
            MultiEraPolicyAssets::ConwayOutput(_, _) => true,
        }
    }

    pub fn is_mint(&self) -> bool {
        match self {
            MultiEraPolicyAssets::AlonzoCompatibleMint(_, _) => true,
            MultiEraPolicyAssets::AlonzoCompatibleOutput(_, _) => false,
            MultiEraPolicyAssets::ConwayMint(_, _) => true,
            MultiEraPolicyAssets::ConwayOutput(_, _) => false,
        }
    }

    pub fn assets(&self) -> Vec<MultiEraAsset> {
        match self {
            MultiEraPolicyAssets::AlonzoCompatibleMint(p, x) => x
                .iter()
                .map(|(k, v)| MultiEraAsset::AlonzoCompatibleMint(p, k, *v))
                .collect(),
            MultiEraPolicyAssets::AlonzoCompatibleOutput(p, x) => x
                .iter()
                .map(|(k, v)| MultiEraAsset::AlonzoCompatibleOutput(p, k, *v))
                .collect(),
            MultiEraPolicyAssets::ConwayMint(p, x) => x
                .iter()
                .map(|(k, v)| MultiEraAsset::ConwayMint(p, k, *v))
                .collect(),
            MultiEraPolicyAssets::ConwayOutput(p, x) => x
                .iter()
                .map(|(k, v)| MultiEraAsset::ConwayOutput(p, k, *v))
                .collect(),
        }
    }

    pub fn collect<'a, T>(&'a self) -> T
    where
        T: FromIterator<(&'a [u8], i128)>,
    {
        match self {
            MultiEraPolicyAssets::AlonzoCompatibleMint(_, x) => {
                x.iter().map(|(k, v)| (k.as_slice(), *v as i128)).collect()
            }
            MultiEraPolicyAssets::AlonzoCompatibleOutput(_, x) => {
                x.iter().map(|(k, v)| (k.as_slice(), *v as i128)).collect()
            }
            MultiEraPolicyAssets::ConwayMint(_, x) => x
                .iter()
                .map(|(k, v)| (k.as_slice(), i64::from(*v) as i128))
                .collect(),
            MultiEraPolicyAssets::ConwayOutput(_, x) => x
                .iter()
                .map(|(k, v)| (k.as_slice(), u64::from(*v) as i128))
                .collect(),
        }
    }
}

impl MultiEraAsset<'_> {
    pub fn policy(&self) -> &Hash<28> {
        match self {
            MultiEraAsset::AlonzoCompatibleMint(x, ..) => x,
            MultiEraAsset::AlonzoCompatibleOutput(x, ..) => x,
            MultiEraAsset::ConwayMint(x, ..) => x,
            MultiEraAsset::ConwayOutput(x, ..) => x,
        }
    }

    pub fn name(&self) -> &[u8] {
        match self {
            MultiEraAsset::AlonzoCompatibleMint(_, x, _) => x,
            MultiEraAsset::AlonzoCompatibleOutput(_, x, _) => x,
            MultiEraAsset::ConwayMint(_, x, _) => x,
            MultiEraAsset::ConwayOutput(_, x, _) => x,
        }
    }

    pub fn is_output(&self) -> bool {
        match self {
            MultiEraAsset::AlonzoCompatibleMint(..) => false,
            MultiEraAsset::AlonzoCompatibleOutput(..) => true,
            MultiEraAsset::ConwayMint(..) => false,
            MultiEraAsset::ConwayOutput(..) => true,
        }
    }

    pub fn is_mint(&self) -> bool {
        match self {
            MultiEraAsset::AlonzoCompatibleMint(..) => true,
            MultiEraAsset::AlonzoCompatibleOutput(..) => false,
            MultiEraAsset::ConwayMint(..) => true,
            MultiEraAsset::ConwayOutput(..) => false,
        }
    }

    pub fn mint_coin(&self) -> Option<i64> {
        match self {
            MultiEraAsset::AlonzoCompatibleMint(_, _, x) => Some(*x),
            MultiEraAsset::AlonzoCompatibleOutput(_, _, _) => None,
            MultiEraAsset::ConwayMint(_, _, x) => Some(x.into()),
            MultiEraAsset::ConwayOutput(_, _, _) => None,
        }
    }

    pub fn output_coin(&self) -> Option<u64> {
        match self {
            MultiEraAsset::AlonzoCompatibleMint(_, _, _) => None,
            MultiEraAsset::AlonzoCompatibleOutput(_, _, x) => Some(*x),
            MultiEraAsset::ConwayMint(_, _, _) => None,
            MultiEraAsset::ConwayOutput(_, _, x) => Some(u64::from(x)),
        }
    }

    pub fn any_coin(&self) -> i128 {
        match self {
            MultiEraAsset::AlonzoCompatibleMint(_, _, x) => *x as i128,
            MultiEraAsset::AlonzoCompatibleOutput(_, _, x) => *x as i128,
            MultiEraAsset::ConwayMint(_, _, x) => i64::from(x) as i128,
            MultiEraAsset::ConwayOutput(_, _, x) => u64::from(x) as i128,
        }
    }

    pub fn to_ascii_name(&self) -> Option<String> {
        let name = self.name();
        String::from_utf8(name.to_vec()).ok()
    }
}
