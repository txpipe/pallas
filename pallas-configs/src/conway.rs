use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenesisFile {
    pub pool_voting_thresholds: PoolVotingThresholds,
    pub d_rep_voting_thresholds: DRepVotingThresholds,
    pub committee_min_size: u64,
    pub committee_max_term_length: u32,
    pub gov_action_lifetime: u32,
    pub gov_action_deposit: u64,
    pub d_rep_deposit: u64,
    pub d_rep_activity: u32,
    pub min_fee_ref_script_cost_per_byte: u64,
    pub plutus_v3_cost_model: Vec<i64>,
    pub constitution: Constitution,
    pub committee: Committee,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PoolVotingThresholds {
    pub committee_normal: f32,
    pub committee_no_confidence: f32,
    pub hard_fork_initiation: f32,
    pub motion_no_confidence: f32,
    pub pp_security_group: f32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DRepVotingThresholds {
    pub motion_no_confidence: f32,
    pub committee_normal: f32,
    pub committee_no_confidence: f32,
    pub update_to_constitution: f32,
    pub hard_fork_initiation: f32,
    pub pp_network_group: f32,
    pub pp_economic_group: f32,
    pub pp_technical_group: f32,
    pub pp_gov_group: f32,
    pub treasury_withdrawal: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Constitution {
    pub anchor: Anchor,
    pub script: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Anchor {
    pub data_hash: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Committee {
    pub members: HashMap<String, u64>,
    pub threshold: Fraction,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Fraction {
    pub numerator: u64,
    pub denominator: u64,
}

impl From<Fraction> for pallas_primitives::conway::RationalNumber {
    fn from(value: Fraction) -> Self {
        Self {
            numerator: value.numerator,
            denominator: value.denominator,
        }
    }
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
            .join(format!("{network}-conway-genesis.json"));

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
