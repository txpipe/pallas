use pallas_crypto::hash::Hash;

use crate::{MultiEraAsset, MultiEraPolicyAssets};

impl<'b> MultiEraPolicyAssets<'b> {
    pub fn policy(&self) -> &Hash<28> {
        match self {
            MultiEraPolicyAssets::AlonzoCompatibleMint(x, _) => x,
            MultiEraPolicyAssets::AlonzoCompatibleOutput(x, _) => x,
        }
    }

    pub fn is_output(&self) -> bool {
        match self {
            MultiEraPolicyAssets::AlonzoCompatibleOutput(_, _) => true,
            MultiEraPolicyAssets::AlonzoCompatibleMint(_, _) => false,
        }
    }

    pub fn is_mint(&self) -> bool {
        match self {
            MultiEraPolicyAssets::AlonzoCompatibleMint(_, _) => true,
            MultiEraPolicyAssets::AlonzoCompatibleOutput(_, _) => false,
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
        }
    }
}

impl<'b> MultiEraAsset<'b> {
    pub fn policy(&self) -> &Hash<28> {
        match self {
            MultiEraAsset::AlonzoCompatibleMint(x, ..) => x,
            MultiEraAsset::AlonzoCompatibleOutput(x, ..) => x,
        }
    }

    pub fn name(&self) -> &[u8] {
        match self {
            MultiEraAsset::AlonzoCompatibleOutput(_, x, _) => x,
            MultiEraAsset::AlonzoCompatibleMint(_, x, _) => x,
        }
    }

    pub fn is_output(&self) -> bool {
        match self {
            MultiEraAsset::AlonzoCompatibleOutput(..) => true,
            MultiEraAsset::AlonzoCompatibleMint(..) => false,
        }
    }

    pub fn is_mint(&self) -> bool {
        match self {
            MultiEraAsset::AlonzoCompatibleMint(..) => true,
            MultiEraAsset::AlonzoCompatibleOutput(..) => false,
        }
    }

    pub fn mint_coin(&self) -> Option<i64> {
        match self {
            MultiEraAsset::AlonzoCompatibleMint(_, _, x) => Some(*x),
            MultiEraAsset::AlonzoCompatibleOutput(_, _, x) => None,
        }
    }

    pub fn output_coin(&self) -> Option<u64> {
        match self {
            MultiEraAsset::AlonzoCompatibleOutput(_, _, x) => Some(*x),
            MultiEraAsset::AlonzoCompatibleMint(_, _, x) => None,
        }
    }

    pub fn any_coin(&self) -> i128 {
        match self {
            MultiEraAsset::AlonzoCompatibleOutput(_, _, x) => *x as i128,
            MultiEraAsset::AlonzoCompatibleMint(_, _, x) => *x as i128,
        }
    }

    pub fn to_ascii_name(&self) -> Option<String> {
        let name = self.name();
        String::from_utf8(name.to_vec()).ok()
    }
}
