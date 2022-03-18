//! Parsing of Byron configuration data

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenesisFile {
    pub avvm_distr: HashMap<String, String>,
    pub block_version_data: BlockVersionData,
    pub fts_seed: String,
    pub protocol_consts: ProtocolConsts,
    pub start_time: u64,
    pub boot_stakeholders: HashMap<String, BootStakeWeight>,
    pub heavy_delegation: HashMap<String, HeavyDelegation>,
    pub non_avvm_balances: HashMap<String, String>,
    pub vss_certs: HashMap<String, VssCert>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockVersionData {
    pub heavy_del_thd: String,
    pub max_block_size: String,
    pub max_header_size: String,
    pub max_proposal_size: String,
    pub max_tx_size: String,
    pub mpc_thd: String,
    pub script_version: u32,
    pub slot_duration: String,
    pub softfork_rule: SoftForkRule,
    pub tx_fee_policy: TxFeePolicy,
    pub unlock_stake_epoch: String,
    pub update_implicit: String,
    pub update_proposal_thd: String,
    pub update_vote_thd: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolConsts {
    pub k: usize,
    pub protocol_magic: u32,
    #[serde(rename = "vssMaxTTL")]
    pub vss_max_ttl: u32,
    #[serde(rename = "vssMinTTL")]
    pub vss_min_ttl: u32,
}

pub type BootStakeWeight = u16;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeavyDelegation {
    pub issuer_pk: String,
    pub delegate_pk: String,
    pub cert: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VssCert {
    pub vss_key: String,
    // TODO: is this size fine?
    pub expiry_epoch: u32,
    pub signature: String,
    pub signing_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftForkRule {
    pub init_thd: String,
    pub min_thd: String,
    pub thd_decrement: String,
}

#[derive(Debug, Deserialize)]
pub struct TxFeePolicy {
    pub multiplier: String,
    pub summand: String,
}

#[cfg(test)]
mod tests {
    use super::GenesisFile;

    #[test]
    fn config_parses_correctly() {
        let json = include_str!("test_data/genesis.json");
        let model: GenesisFile = serde_json::from_str(json).unwrap();

        assert_eq!(model.avvm_distr.len(), 100);
        assert_eq!(model.protocol_consts.k, 2160);
        assert_eq!(model.start_time, 1563999616);
        assert_eq!(
            model.fts_seed,
            "76617361206f7061736120736b6f766f726f64612047677572646120626f726f64612070726f766f6461"
        );
    }
}
