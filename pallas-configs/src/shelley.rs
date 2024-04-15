use num_rational::BigRational;
use pallas_crypto::hash::Hash;
use pallas_primitives::conway::RationalNumber;
use serde::{Deserialize, Deserializer};
use std::{collections::HashMap, str::FromStr};

fn deserialize_rational<'de, D>(
    deserializer: D,
) -> Result<pallas_primitives::alonzo::RationalNumber, D::Error>
where
    D: Deserializer<'de>,
{
    let s = f32::deserialize(deserializer)?;
    let r = BigRational::from_float(s).unwrap();
    let r = pallas_primitives::alonzo::RationalNumber {
        numerator: r.numer().try_into().unwrap(),
        denominator: r.denom().try_into().unwrap(),
    };

    Ok(r)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenDelegs {
    pub delegate: Option<String>,
    pub vrf: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolVersion {
    pub minor: u64,
    pub major: u64,
}

impl From<ProtocolVersion> for pallas_primitives::alonzo::ProtocolVersion {
    fn from(value: ProtocolVersion) -> Self {
        (value.major, value.minor)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub enum NonceVariant {
    NeutralNonce,
    Nonce,
}

impl From<NonceVariant> for pallas_primitives::alonzo::NonceVariant {
    fn from(value: NonceVariant) -> Self {
        match value {
            NonceVariant::NeutralNonce => Self::NeutralNonce,
            NonceVariant::Nonce => Self::Nonce,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExtraEntropy {
    pub tag: NonceVariant,
    pub hash: Option<String>,
}

impl From<ExtraEntropy> for pallas_primitives::alonzo::Nonce {
    fn from(value: ExtraEntropy) -> Self {
        Self {
            variant: value.tag.into(),
            hash: value
                .hash
                .map(|x| Hash::<32>::from_str(&x).expect("invalid nonce hash value")),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolParams {
    pub protocol_version: ProtocolVersion,
    pub max_tx_size: u32,
    pub max_block_body_size: u32,
    pub max_block_header_size: u32,
    pub key_deposit: u64,
    #[serde(rename = "minUTxOValue")]
    pub min_utxo_value: u64,
    pub min_fee_a: u32,
    pub min_fee_b: u32,
    pub pool_deposit: u64,
    pub n_opt: u32,
    pub min_pool_cost: u64,
    pub e_max: u32,
    pub extra_entropy: ExtraEntropy,

    #[serde(deserialize_with = "deserialize_rational")]
    pub decentralisation_param: RationalNumber,

    #[serde(deserialize_with = "deserialize_rational")]
    pub rho: pallas_primitives::alonzo::RationalNumber,

    #[serde(deserialize_with = "deserialize_rational")]
    pub tau: pallas_primitives::alonzo::RationalNumber,

    #[serde(deserialize_with = "deserialize_rational")]
    pub a0: pallas_primitives::alonzo::RationalNumber,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Staking {
    pub pools: Option<HashMap<String, String>>,
    pub stake: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenesisFile {
    pub active_slots_coeff: Option<f32>,
    pub epoch_length: Option<u32>,
    pub gen_delegs: Option<HashMap<String, GenDelegs>>,
    pub initial_funds: Option<HashMap<String, String>>,
    pub max_kes_evolutions: Option<u32>,
    pub max_lovelace_supply: Option<u64>,
    pub network_id: Option<String>,
    pub network_magic: Option<u32>,
    pub protocol_params: ProtocolParams,
    pub security_param: Option<u32>,
    pub slot_length: Option<u32>,
    pub slots_per_kes_period: Option<u32>,
    pub staking: Option<Staking>,
    pub system_start: Option<String>,
    pub update_quorum: Option<u32>,
}

pub fn from_file(path: &std::path::Path) -> Result<GenesisFile, std::io::Error> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let parsed: GenesisFile = serde_json::from_reader(reader)?;

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_test_data_config(network: &str) -> GenesisFile {
        let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("test_data")
            .join(format!("{network}-shelley-genesis.json"));

        from_file(&path).unwrap()
    }

    #[test]
    fn test_preview_json_loads() {
        load_test_data_config("preview");
    }

    #[test]
    fn test_mainnet_json_loads() {
        load_test_data_config("mainnet");
    }
}
