//! Parsing of Byron configuration data

use pallas_addresses::ByronAddress;
use pallas_crypto::hash::Hash;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenesisFile {
    pub avvm_distr: HashMap<String, String>,
    pub block_version_data: BlockVersionData,
    pub fts_seed: Option<String>,
    pub protocol_consts: ProtocolConsts,
    pub start_time: u64,
    pub boot_stakeholders: HashMap<String, BootStakeWeight>,
    pub heavy_delegation: HashMap<String, HeavyDelegation>,
    pub non_avvm_balances: HashMap<String, String>,
    pub vss_certs: Option<HashMap<String, VssCert>>,
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
    pub vss_max_ttl: Option<u32>,
    #[serde(rename = "vssMinTTL")]
    pub vss_min_ttl: Option<u32>,
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

pub fn from_file(path: &std::path::Path) -> Result<GenesisFile, std::io::Error> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let parsed: GenesisFile = serde_json::from_reader(reader)?;

    Ok(parsed)
}

use base64::Engine;

pub fn genesis_avvm_utxos(config: &GenesisFile) -> Vec<(Hash<32>, u64)> {
    config
        .avvm_distr
        .iter()
        .map(|(pubkey, amount)| {
            let amount = amount.parse().unwrap();
            let pubkey = base64::engine::general_purpose::URL_SAFE
                .decode(pubkey)
                .unwrap();

            let pubkey = pallas_crypto::key::ed25519::PublicKey::try_from(&pubkey[..]).unwrap();

            // TODO: network tag
            //let network_tag = Some(config.protocol_consts.protocol_magic);
            let network_tag = None;

            let addr: pallas_addresses::ByronAddress =
                pallas_addresses::byron::AddressPayload::new_redeem(pubkey, network_tag).into();

            let txid = pallas_crypto::hash::Hasher::<256>::hash_cbor(&addr);

            (txid, amount)
        })
        .collect()
}

pub fn genesis_non_avvm_utxos(config: &GenesisFile) -> Vec<(Hash<32>, u64)> {
    config
        .non_avvm_balances
        .iter()
        .map(|(addr, amount)| {
            let amount = amount.parse().unwrap();
            let addr = ByronAddress::from_base58(addr).unwrap();

            let txid = pallas_crypto::hash::Hasher::<256>::hash_cbor(&addr);

            (txid, amount)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAINNET_GENESIS_AVVM_PUBKEY: &str = &"-Eot4a-P3RKYYdZwisLhe7iflHhy9H6JwCsizjT0UQE=";

    #[test]
    pub fn test_preview_json_loads() {
        let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("test_data")
            .join("preview-byron-genesis.json");

        println!("{:?}", path);

        let f = from_file(&path).unwrap();
    }

    #[test]
    pub fn test_preview_non_avvm_utxos() {
        let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("test_data")
            .join("preview-byron-genesis.json");

        let f = from_file(&path).unwrap();

        let utxos = super::genesis_non_avvm_utxos(&f);

        dbg!(utxos);
    }

    #[test]
    pub fn test_mainnet_avvm_utxos() {
        let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("test_data")
            .join("mainnet-byron-genesis.json");

        let f = from_file(&path).unwrap();

        let utxos = super::genesis_avvm_utxos(&f);

        for (hash, _) in utxos {
            let hs = format!("{}", hash);
            assert_ne!(
                hs,
                "3a33ff3e51cf2a67b973945442c35181d5a21b6c657d760acba62f48ad7d1d69"
            );
        }
    }
}
