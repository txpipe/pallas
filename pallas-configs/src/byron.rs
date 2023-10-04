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

pub type GenesisUtxo = (Hash<32>, ByronAddress, u64);

pub fn genesis_avvm_utxos(config: &GenesisFile) -> Vec<GenesisUtxo> {
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

            let addr = pallas_addresses::byron::AddressPayload::new_redeem(pubkey, network_tag);

            let addr: pallas_addresses::ByronAddress = addr.into();

            let txid = pallas_crypto::hash::Hasher::<256>::hash_cbor(&addr);

            (txid, addr, amount)
        })
        .collect()
}

pub fn genesis_non_avvm_utxos(config: &GenesisFile) -> Vec<GenesisUtxo> {
    config
        .non_avvm_balances
        .iter()
        .map(|(addr, amount)| {
            let amount = amount.parse().unwrap();
            let addr = ByronAddress::from_base58(addr).unwrap();

            let txid = pallas_crypto::hash::Hasher::<256>::hash_cbor(&addr);

            (txid, addr, amount)
        })
        .collect()
}

pub fn genesis_utxos(config: &GenesisFile) -> Vec<GenesisUtxo> {
    let avvm = genesis_avvm_utxos(config);
    let non_avvm = genesis_non_avvm_utxos(config);

    [avvm, non_avvm].concat().to_vec()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn load_test_data_config(network: &str) -> GenesisFile {
        let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("test_data")
            .join(format!("{network}-byron-genesis.json"));

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

    fn utxo_exists(set: &[GenesisUtxo], expected: GenesisUtxo) -> bool {
        set.iter().any(|(hash, addr, amount)| {
            hash.eq(&expected.0) && addr.eq(&expected.1) && amount.eq(&expected.2)
        })
    }

    fn genesis_utxo_from_raw(hash_hex: &str, addr_base58: &str, amount: u64) -> GenesisUtxo {
        (
            Hash::from_str(hash_hex).unwrap(),
            ByronAddress::from_base58(addr_base58).unwrap(),
            amount,
        )
    }

    #[test]
    fn test_preview_non_avvm_utxos() {
        let f = load_test_data_config("preview");

        let utxos = super::genesis_non_avvm_utxos(&f);
        assert_eq!(utxos.len(), 8);

        // check known tx as seen: https://preview.cexplorer.io/tx/4843cf2e582b2f9ce37600e5ab4cc678991f988f8780fed05407f9537f7712bd
        let expected = genesis_utxo_from_raw(
            "4843cf2e582b2f9ce37600e5ab4cc678991f988f8780fed05407f9537f7712bd",
            "FHnt4NL7yPXvDWHa8bVs73UEUdJd64VxWXSFNqetECtYfTd9TtJguJ14Lu3feth",
            30_000_000_000_000_000,
        );

        assert!(utxo_exists(&utxos, expected));
    }

    #[test]
    pub fn test_mainnet_avvm_utxos() {
        let f = load_test_data_config("mainnet");

        let utxos = super::genesis_non_avvm_utxos(&f);

        // there aren't non-avvm utxos in mainnet
        assert!(utxos.is_empty());

        let utxos = super::genesis_avvm_utxos(&f);

        assert_eq!(utxos.len(), 14505);

        // check known tx as seen: https://cexplorer.io/tx/0ae3da29711600e94a33fb7441d2e76876a9a1e98b5ebdefbf2e3bc535617616
        let expected = genesis_utxo_from_raw(
            "0ae3da29711600e94a33fb7441d2e76876a9a1e98b5ebdefbf2e3bc535617616",
            "Ae2tdPwUPEZKQuZh2UndEoTKEakMYHGNjJVYmNZgJk2qqgHouxDsA5oT83n",
            2_463_071_701_000_000,
        );

        assert!(utxo_exists(&utxos, expected));
    }
}
