use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenDelegs {
    pub delegate: Option<String>,
    pub vrf: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolVersion {
    pub minor: Option<u32>,
    pub major: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraEntropy {
    pub tag: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolParams {
    pub protocol_version: Option<ProtocolVersion>,
    pub decentralisation_param: Option<u32>,
    pub e_max: Option<u32>,
    pub extra_entropy: Option<ExtraEntropy>,
    pub max_tx_size: Option<u32>,
    pub max_block_body_size: Option<u32>,
    pub max_block_header_size: Option<u32>,
    pub min_fee_a: Option<u32>,
    pub min_fee_b: Option<u32>,
    pub min_u_tx_o_value: Option<u32>,
    pub pool_deposit: Option<u64>,
    pub min_pool_cost: Option<u64>,
    pub key_deposit: Option<u32>,
    pub n_opt: Option<u32>,
    pub rho: Option<f32>,
    pub tau: Option<f32>,
    pub a0: Option<f32>,
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
    pub protocol_params: Option<ProtocolParams>,
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
