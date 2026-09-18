use crate::injection::{self, Injection};
use num_rational::Rational64;
use pallas_crypto::hash::Hash;
use pallas_primitives::conway::{Epoch, RationalNumber};
use serde::{Deserialize, Deserializer};
use std::{collections::HashMap, str::FromStr};

fn deserialize_rational<'de, D>(
    deserializer: D,
) -> Result<pallas_primitives::alonzo::RationalNumber, D::Error>
where
    D: Deserializer<'de>,
{
    let s = f32::deserialize(deserializer)?;
    let r = Rational64::approximate_float(s)
        .ok_or(serde::de::Error::custom("can't turn float into rational"))?;

    let r = pallas_primitives::alonzo::RationalNumber {
        numerator: *r.numer() as u64,
        denominator: *r.denom() as u64,
    };

    Ok(r)
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
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
    pub e_max: Epoch,
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

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub hash: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SingleHostAddr {
    pub port: Option<u32>,
    #[serde(rename = "IPv6")]
    pub ipv6: Option<String>,
    #[serde(rename = "IPv4")]
    pub ipv4: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SingleHostName {
    pub port: Option<u32>,
    pub dns_name: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MultiHostName {
    pub dns_name: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Relay {
    SingleHostAddr(SingleHostAddr),
    SingleHostName(SingleHostName),
    MultiHostName(MultiHostName),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Credential {
    KeyHash(String),
    ScriptHash(String),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RewardAccount {
    pub credential: Credential,
    pub network: String,
}

/// A pool entry as written, before the current and legacy names for its
/// id and reward account are reconciled.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoolRaw {
    cost: u64,
    #[serde(deserialize_with = "deserialize_rational")]
    margin: pallas_primitives::alonzo::RationalNumber,
    metadata: Option<Metadata>,
    #[serde(default)]
    owners: Vec<String>,
    pledge: u64,
    pool_id: Option<String>,
    public_key: Option<String>,
    relays: Vec<HashMap<String, Relay>>,
    account_address: Option<RewardAccount>,
    reward_account: Option<RewardAccount>,
    vrf: String,
    #[serde(default)]
    registration_deposit: Option<u64>,
}

impl TryFrom<PoolRaw> for Pool {
    type Error = String;

    fn try_from(raw: PoolRaw) -> Result<Self, Self::Error> {
        // The ledger accepts either name and prefers `poolId`. A serde alias
        // would reject a pool carrying both as a duplicate field, so the
        // fallback is written out by hand.
        let public_key = raw.pool_id.or(raw.public_key).ok_or_else(|| {
            "a pool names its identifier neither as poolId nor as publicKey".to_string()
        })?;

        let reward_account = raw.account_address.or(raw.reward_account).ok_or_else(|| {
            "a pool names its reward account neither as accountAddress nor as rewardAccount"
                .to_string()
        })?;

        Ok(Self {
            cost: raw.cost,
            margin: raw.margin,
            metadata: raw.metadata,
            owners: raw.owners,
            pledge: raw.pledge,
            public_key,
            relays: raw.relays,
            reward_account,
            vrf: raw.vrf,
            registration_deposit: raw.registration_deposit,
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(try_from = "PoolRaw")]
pub struct Pool {
    pub cost: u64,
    pub margin: pallas_primitives::alonzo::RationalNumber,
    pub metadata: Option<Metadata>,
    pub owners: Vec<String>,
    pub pledge: u64,
    pub public_key: String, // pool ID, written as `poolId`
    pub relays: Vec<HashMap<String, Relay>>,
    pub reward_account: RewardAccount, // written as `accountAddress`
    pub vrf: String,
    pub registration_deposit: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Staking {
    pub pools: Option<HashMap<String, Pool>>,
    pub stake: Option<HashMap<String, String>>,
}

/// The Shelley genesis fields a generator writes under `extraConfig` rather
/// than at the top level.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtraConfig {
    pub initial_funds: Option<Injection<HashMap<String, u64>>>,
    pub stake_pools: Option<Injection<HashMap<String, Pool>>>,
    pub stake_credentials: Option<Injection<HashMap<String, String>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenesisFileRaw {
    active_slots_coeff: Option<f32>,
    epoch_length: Option<u32>,
    gen_delegs: Option<HashMap<String, GenDelegs>>,
    initial_funds: Option<HashMap<String, u64>>,
    max_lovelace_supply: Option<u64>,
    network_id: Option<String>,
    network_magic: Option<u32>,
    protocol_params: ProtocolParams,
    security_param: Option<u32>,
    slot_length: Option<u32>,
    staking: Option<Staking>,
    system_start: Option<String>,
    update_quorum: Option<u32>,
    extra_config: Option<ExtraConfig>,

    #[serde(rename = "maxKESEvolutions")]
    max_kes_evolutions: Option<u32>,

    #[serde(rename = "slotsPerKESPeriod")]
    slots_per_kes_period: Option<u32>,
}

impl TryFrom<GenesisFileRaw> for GenesisFile {
    type Error = String;

    fn try_from(raw: GenesisFileRaw) -> Result<Self, Self::Error> {
        raw.fold(&injection::Source::NoFilesystem)
    }
}

impl GenesisFileRaw {
    fn fold(self, source: &injection::Source) -> Result<GenesisFile, String> {
        let extra = self.extra_config.unwrap_or_default();

        let had_staking = self.staking.is_some();
        let (top_level_pools, top_level_stake) = match self.staking {
            Some(staking) => (staking.pools, staking.stake),
            None => (None, None),
        };

        let initial_funds = injection::resolve(
            source,
            "initialFunds",
            "initialFunds",
            extra.initial_funds,
            self.initial_funds,
        )?;

        let pools = injection::resolve(
            source,
            "stakePools",
            "staking.pools",
            extra.stake_pools,
            top_level_pools,
        )?;

        let stake = injection::resolve(
            source,
            "stakeCredentials",
            "staking.stake",
            extra.stake_credentials,
            top_level_stake,
        )?;

        let staking = if had_staking || pools.is_some() || stake.is_some() {
            Some(Staking { pools, stake })
        } else {
            None
        };

        Ok(GenesisFile {
            active_slots_coeff: self.active_slots_coeff,
            epoch_length: self.epoch_length,
            gen_delegs: self.gen_delegs,
            initial_funds,
            max_lovelace_supply: self.max_lovelace_supply,
            network_id: self.network_id,
            network_magic: self.network_magic,
            protocol_params: self.protocol_params,
            security_param: self.security_param,
            slot_length: self.slot_length,
            staking,
            system_start: self.system_start,
            update_quorum: self.update_quorum,
            max_kes_evolutions: self.max_kes_evolutions,
            slots_per_kes_period: self.slots_per_kes_period,
        })
    }
}

/// A parsed Shelley genesis file.
///
/// A parsed shelley genesis. Funds, pools and delegations written under
/// `extraConfig` are already merged into `initial_funds` and `staking`.
#[derive(Debug, Deserialize, Clone)]
#[serde(try_from = "GenesisFileRaw")]
pub struct GenesisFile {
    pub active_slots_coeff: Option<f32>,
    pub epoch_length: Option<u32>,
    pub gen_delegs: Option<HashMap<String, GenDelegs>>,
    pub initial_funds: Option<HashMap<String, u64>>,
    pub max_lovelace_supply: Option<u64>,
    pub network_id: Option<String>,
    pub network_magic: Option<u32>,
    pub protocol_params: ProtocolParams,
    pub security_param: Option<u32>,
    pub slot_length: Option<u32>,
    pub staking: Option<Staking>,
    pub system_start: Option<String>,
    pub update_quorum: Option<u32>,
    pub max_kes_evolutions: Option<u32>,
    pub slots_per_kes_period: Option<u32>,
}

pub fn from_file(path: &std::path::Path) -> Result<GenesisFile, std::io::Error> {
    let text = std::fs::read_to_string(path)?;
    let raw: GenesisFileRaw = serde_json::from_str(&text)?;

    // The segments are joined under the directory of the genesis file.
    let directory = path
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""))
        .to_path_buf();

    raw.fold(&injection::Source::Directory(directory))
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
}

pub type GenesisUtxo = (Hash<32>, pallas_addresses::Address, u64);

pub fn shelley_utxos(config: &GenesisFile) -> Vec<GenesisUtxo> {
    match &config.initial_funds {
        None => Vec::new(),
        Some(funds) => funds
            .iter()
            .map(|(addr, amount)| {
                let addr = pallas_addresses::Address::from_hex(addr).unwrap();

                let txid = pallas_crypto::hash::Hasher::<256>::hash(&addr.to_vec());

                (txid, addr, *amount)
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INJECTED_POOL: &str = "9c70bd513f16961b3debef699da1fb8c77138edd149f00f9b3b5522d";
    const INJECTED_DELEGATOR: &str = "ae1537988b77a8815a7502eb6e02026bc71d0a7729cddc2a21582980";
    const INJECTED_VRF: &str = "63986178c45c411fb4ab0fe4f8717a102168ceb6727bcfec68edf577c08349bd";
    const INJECTED_CREDENTIAL: &str = "8c2bc8429a2fb885c1b17a8095b927cfcfe97e4fa5fe83f0dd4418ab";

    fn test_data_path(network: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("test_data")
            .join(format!("{network}-shelley-genesis.json"))
    }

    fn load_test_data_config(network: &str) -> GenesisFile {
        from_file(&test_data_path(network)).unwrap()
    }

    fn test_data_json(network: &str) -> serde_json::Value {
        let text = std::fs::read_to_string(test_data_path(network)).unwrap();

        serde_json::from_str(&text).unwrap()
    }

    fn injected_funds() -> HashMap<String, u64> {
        [
            (
                "00fbd8c60336e0f03852209dcb827dd30e789ee80e6d104369097a3cb2ae1537988b77a8815a7502eb6e02026bc71d0a7729cddc2a21582980",
                4_500_000_000_000,
            ),
            (
                "6004d2cf712cfcaafb8bda85dc31baf3a35168d2e28029e0b56c562d37",
                2_250_000_000_000,
            ),
            (
                "607d1ca70e7e84c99e23c3f9494bc48a9fe30e98e02e25745f96d7a731",
                2_250_000_000_000,
            ),
        ]
        .into_iter()
        .map(|(address, amount)| (address.to_string(), amount))
        .collect()
    }

    fn sorted_keys(value: &serde_json::Value) -> Vec<String> {
        let mut keys: Vec<String> = value
            .as_object()
            .expect("expected a JSON object")
            .keys()
            .cloned()
            .collect();
        keys.sort();

        keys
    }

    fn pool_json(id_key: &str, account_key: &str) -> String {
        format!(
            r#"{{
                "{id_key}": "{INJECTED_POOL}",
                "{account_key}": {{
                    "credential": {{
                        "keyHash": "{INJECTED_CREDENTIAL}"
                    }},
                    "network": "Testnet"
                }},
                "cost": 0,
                "margin": 0,
                "metadata": null,
                "owners": [],
                "pledge": 0,
                "relays": [],
                "vrf": "{INJECTED_VRF}"
            }}"#
        )
    }

    #[test]
    fn calc_address_txid() {
        let config = load_test_data_config("golden");
        let utxos = shelley_utxos(&config);
        let utxo = utxos.first().unwrap();
        assert_eq!(
            utxo.0.to_string(),
            "f9ec23569778d1c5f7f43e0e98464335f02fb98b57683faa1c6b18c82921d2da"
        );
        assert_eq!(
            utxo.1.to_bech32().unwrap(),
            "addr_test1qrsm4h32h9r95f8at64ykuugxqu3wvu0s5ay3vg6tlyevjh4e2flkegka00r69gt8c4vkxgf2vnnph3nsvhlkg5ukgxslee3tf"
        );
        assert_eq!(utxo.2, 12157196);
    }

    #[test]
    fn test_preview_json_loads() {
        load_test_data_config("preview");
    }

    #[test]
    fn test_mainnet_json_loads() {
        load_test_data_config("mainnet");
    }

    #[test]
    fn test_musashi_json_loads() {
        let json = test_data_json("musashi");
        assert!(json["initialFunds"].as_object().unwrap().is_empty());

        let config = load_test_data_config("musashi");
        let total: u64 = shelley_utxos(&config).iter().map(|(_, _, v)| v).sum();
        assert_eq!(total, 30000000900000000);
    }

    #[test]
    fn test_partner_staking_parses() {
        let config = load_test_data_config("partner");

        let staking = config
            .staking
            .expect("staking section must be present in partner genesis");

        let pools = staking
            .pools
            .expect("staking pools should be available in partner genesis");
        assert_eq!(pools.len(), 3);

        let pool = pools
            .get("2d0de269b0996fdcd8f19f0b6d7d0bf14363984482f181a5a1ccd036")
            .expect("expected initial pool registration");

        assert_eq!(pool.cost, 0);
        assert_eq!(pool.pledge, 0);
        assert_eq!(pool.owners.len(), 0);
        assert_eq!(pool.relays.len(), 0);
        assert_eq!(pool.reward_account.network, "Testnet");
        match &pool.reward_account.credential {
            Credential::KeyHash(key) => assert_eq!(
                key,
                "f0834f87e577f514cecdd41db9feabcc42671b4b15cd5a27924e571c"
            ),
            _ => panic!("expected key hash credential"),
        }
        assert_eq!(
            pool.vrf,
            "72255c577e9fa3146e397ffeb45a187c48505a5f950216d7b1224d85dc4fbbac"
        );
        assert_eq!(pool.margin.numerator, 0);

        let stake = staking
            .stake
            .expect("delegation map should exist in partner genesis");
        assert_eq!(stake.len(), 3);
        assert_eq!(
            stake
                .get("f441c3ef7ef4a8ade039f4f4224b9ae494125bbcff284df64e8e73d8")
                .expect("stake delegation should exist"),
            "2d0de269b0996fdcd8f19f0b6d7d0bf14363984482f181a5a1ccd036"
        );
    }

    #[test]
    fn injected_funds_reach_the_utxo_set() {
        let value = test_data_json("generated");

        assert!(
            value["initialFunds"]
                .as_object()
                .expect("the fixture must carry a top level initialFunds object")
                .is_empty(),
            "the fixture's top level funds must be empty, or this says nothing about the injection"
        );

        let injected: HashMap<String, u64> =
            serde_json::from_value(value["extraConfig"]["initialFunds"]["data"].clone())
                .expect("the fixture must carry an injected fund payload");

        assert_eq!(
            injected,
            injected_funds(),
            "the fixture must inject the funds this case was written against"
        );

        let config = load_test_data_config("generated");

        assert_eq!(
            config.initial_funds.as_ref(),
            Some(&injected),
            "the folded funds must be exactly the injected payload"
        );

        let reached: HashMap<String, u64> = shelley_utxos(&config)
            .into_iter()
            .map(|(_, address, amount)| (address.to_hex(), amount))
            .collect();

        assert_eq!(
            reached, injected,
            "every injected fund must reach the utxo set under its own address"
        );
    }

    #[test]
    fn injected_staking_reaches_the_accessor() {
        let value = test_data_json("generated");

        for field in ["pools", "stake"] {
            assert!(
                value["staking"][field]
                    .as_object()
                    .unwrap_or_else(|| panic!("the fixture must carry a top level staking.{field}"))
                    .is_empty(),
                "the fixture's top level staking.{field} must be empty, or this says nothing about the injection"
            );
        }

        let injected_pools: Vec<String> = sorted_keys(&value["extraConfig"]["stakePools"]["data"]);
        let injected_stake: HashMap<String, String> =
            serde_json::from_value(value["extraConfig"]["stakeCredentials"]["data"].clone())
                .expect("the fixture must carry an injected credential payload");

        assert_eq!(
            injected_pools,
            vec![INJECTED_POOL.to_string()],
            "the fixture must inject the pool this case was written against"
        );
        assert_eq!(
            injected_stake.get(INJECTED_DELEGATOR),
            Some(&INJECTED_POOL.to_string()),
            "the fixture must inject the delegation this case was written against"
        );

        let config = load_test_data_config("generated");

        let staking = config.staking.expect("staking must be present");
        let pools = staking.pools.expect("pools must be present");
        let stake = staking.stake.expect("delegations must be present");

        let mut reached: Vec<String> = pools.keys().cloned().collect();
        reached.sort();

        assert_eq!(
            reached, injected_pools,
            "the folded pools must be exactly the injected ones"
        );
        assert_eq!(
            stake, injected_stake,
            "the folded delegations must be exactly the injected payload"
        );

        let pool = pools
            .get(INJECTED_POOL)
            .expect("the injected pool must be present");

        assert_eq!(pool.public_key, INJECTED_POOL);
        assert_eq!(pool.vrf, INJECTED_VRF);
        assert_eq!(pool.reward_account.network, "Testnet");

        match &pool.reward_account.credential {
            Credential::KeyHash(key) => assert_eq!(key, INJECTED_CREDENTIAL),
            _ => panic!("expected a key hash credential"),
        }
    }

    #[test]
    fn an_injection_file_reaches_the_utxo_set() {
        let value = test_data_json("file-injection");

        assert_eq!(
            value["extraConfig"]["initialFunds"]["file"],
            serde_json::json!(["file-injection", "initial-funds.json"]),
            "the fixture must name the injection file this case was written against"
        );
        assert!(
            value["initialFunds"]
                .as_object()
                .expect("the fixture must carry a top level initialFunds object")
                .is_empty(),
            "the fixture's top level funds must be empty, or this says nothing about the injection"
        );

        let config = load_test_data_config("file-injection");

        assert_eq!(
            config.initial_funds.as_ref(),
            Some(&injected_funds()),
            "the funds read from the injection file must be the inline fixture's funds"
        );

        let reached: HashMap<String, u64> = shelley_utxos(&config)
            .into_iter()
            .map(|(_, address, amount)| (address.to_hex(), amount))
            .collect();

        assert_eq!(
            reached,
            injected_funds(),
            "every fund read from the file must reach the utxo set under its own address"
        );
    }

    #[test]
    fn the_parse_path_still_refuses_an_injection_file() {
        let text = std::fs::read_to_string(test_data_path("file-injection")).unwrap();

        let err = serde_json::from_str::<GenesisFile>(&text)
            .expect_err("a genesis parsed from text alone must refuse an injection file");

        assert!(err.to_string().contains("initialFunds"), "{err}");
        assert!(
            err.to_string()
                .contains("file-injection/initial-funds.json"),
            "{err}"
        );
        assert!(
            err.to_string().contains("cannot be read while parsing"),
            "{err}"
        );
    }

    #[test]
    fn an_empty_payload_against_a_populated_top_level_is_refused() {
        let mut value = test_data_json("generated");
        let funds = value["extraConfig"]["initialFunds"]["data"].take();
        value["initialFunds"] = funds;
        value["extraConfig"]["initialFunds"] = serde_json::json!({ "data": {} });

        let err = serde_json::from_value::<GenesisFile>(value)
            .expect_err("an empty payload against a populated top level must be refused");

        assert!(err.to_string().contains("both populated"), "{err}");
    }

    #[test]
    fn an_unmodelled_extra_config_key_is_refused() {
        let mut value = test_data_json("generated");
        value["extraConfig"]["initialDReps"] = serde_json::json!({ "data": {} });

        let err = serde_json::from_value::<GenesisFile>(value)
            .expect_err("an unmodelled extraConfig key must be refused");

        assert!(err.to_string().contains("unknown field"), "{err}");
        assert!(err.to_string().contains("initialDReps"), "{err}");
    }

    #[test]
    fn a_pool_parses_under_the_current_names() {
        let pool: Pool =
            serde_json::from_str(&pool_json("poolId", "accountAddress")).expect("pool must parse");

        assert_eq!(pool.public_key, INJECTED_POOL);
        assert_eq!(pool.reward_account.network, "Testnet");
    }

    #[test]
    fn a_pool_parses_under_the_older_names() {
        let pool: Pool = serde_json::from_str(&pool_json("publicKey", "rewardAccount"))
            .expect("pool must parse");

        assert_eq!(pool.public_key, INJECTED_POOL);
        assert_eq!(pool.reward_account.network, "Testnet");
    }

    #[test]
    fn a_field_named_neither_way_is_refused() {
        let err = serde_json::from_str::<Pool>(&pool_json("poolIdentifier", "accountAddress"))
            .expect_err("a pool with no identifier must be refused");

        assert!(err.to_string().contains("poolId"), "{err}");
        assert!(err.to_string().contains("publicKey"), "{err}");

        let err = serde_json::from_str::<Pool>(&pool_json("poolId", "account"))
            .expect_err("a pool with no reward account must be refused");

        assert!(err.to_string().contains("accountAddress"), "{err}");
        assert!(err.to_string().contains("rewardAccount"), "{err}");
    }

    // The two values differ on purpose, so the assertion says which spelling
    // won rather than only that one did.
    #[test]
    fn both_spellings_prefer_the_current_name() {
        let mut value: serde_json::Value =
            serde_json::from_str(&pool_json("poolId", "accountAddress")).unwrap();
        value["publicKey"] =
            serde_json::json!("0000000000000000000000000000000000000000000000000000000f");
        value["rewardAccount"] = serde_json::json!({
            "credential": {
                "keyHash": "00000000000000000000000000000000000000000000000000000001"
            },
            "network": "Mainnet"
        });

        let pool: Pool =
            serde_json::from_value(value).expect("a pool with both spellings must parse");

        assert_eq!(pool.public_key, INJECTED_POOL);
        assert_eq!(pool.reward_account.network, "Testnet");

        match &pool.reward_account.credential {
            Credential::KeyHash(key) => assert_eq!(key, INJECTED_CREDENTIAL),
            _ => panic!("expected a key hash credential"),
        }
    }

    // The counts are named as well as the maps, because a comparison drawn
    // entirely from the file moves whenever the file moves.
    #[test]
    fn a_genesis_without_extra_config_is_untouched() {
        for (network, fund_count, pool_count) in [("golden", 1, 1), ("partner", 4, 3)] {
            let value = test_data_json(network);

            assert!(
                value.get("extraConfig").is_none(),
                "{network} must carry no extraConfig, or it is the wrong fixture for this case"
            );

            let funds: HashMap<String, u64> = serde_json::from_value(value["initialFunds"].clone())
                .unwrap_or_else(|err| panic!("{network} initialFunds must parse: {err}"));

            assert_eq!(funds.len(), fund_count, "{network} top level fund count");

            let config = load_test_data_config(network);

            assert_eq!(
                config.initial_funds.as_ref(),
                Some(&funds),
                "{network} funds must be exactly its own top level map"
            );

            let pools = sorted_keys(&value["staking"]["pools"]);
            assert_eq!(pools.len(), pool_count, "{network} top level pool count");

            let staking = config
                .staking
                .unwrap_or_else(|| panic!("{network} staking must be present"));
            let mut reached: Vec<String> = staking
                .pools
                .unwrap_or_else(|| panic!("{network} pools must be present"))
                .keys()
                .cloned()
                .collect();
            reached.sort();

            assert_eq!(
                reached, pools,
                "{network} pools must be exactly its own top level map"
            );
        }
    }

    #[test]
    fn an_absent_injection_leaves_the_top_level() {
        let mut value = test_data_json("generated");
        let funds = value["extraConfig"]["initialFunds"]["data"].take();
        value["initialFunds"] = funds;
        value["extraConfig"]["initialFunds"] = serde_json::json!({});

        let config: GenesisFile =
            serde_json::from_value(value).expect("the genesis must still parse");

        assert_eq!(shelley_utxos(&config).len(), 3);
    }

    // The top-level pool is the injected one under a different identifier and
    // in the older spelling, so only the two populated maps carry the refusal.
    #[test]
    fn pools_in_both_places_are_refused() {
        let mut value = test_data_json("generated");
        let mut pool = value["extraConfig"]["stakePools"]["data"][INJECTED_POOL].clone();

        let entry = pool.as_object_mut().expect("the pool must be an object");
        let id = entry.remove("poolId").expect("the pool must carry an id");
        entry.insert("publicKey".to_string(), id);
        let account = entry
            .remove("accountAddress")
            .expect("the pool must carry an account");
        entry.insert("rewardAccount".to_string(), account);

        value["staking"]["pools"] = serde_json::json!({
            "0000000000000000000000000000000000000000000000000000000f": pool
        });

        let err = serde_json::from_value::<GenesisFile>(value)
            .expect_err("a pool map with two sources must be refused");

        assert!(err.to_string().contains("stakePools"), "{err}");
        assert!(err.to_string().contains("both populated"), "{err}");
    }

    #[test]
    fn funds_in_both_places_are_refused() {
        let mut value = test_data_json("generated");
        value["initialFunds"] = serde_json::json!({
            "6000000000000000000000000000000000000000000000000000000001": 1
        });

        let err = serde_json::from_value::<GenesisFile>(value)
            .expect_err("funds with two sources must be refused");

        assert!(err.to_string().contains("initialFunds"), "{err}");
        assert!(err.to_string().contains("both populated"), "{err}");
    }

    #[test]
    fn credentials_in_both_places_are_refused() {
        let mut value = test_data_json("generated");
        value["staking"]["stake"] = serde_json::json!({
            "0000000000000000000000000000000000000000000000000000000e":
                "0000000000000000000000000000000000000000000000000000000f"
        });

        let err = serde_json::from_value::<GenesisFile>(value)
            .expect_err("stake credentials with two sources must be refused");

        assert!(err.to_string().contains("stakeCredentials"), "{err}");
        assert!(err.to_string().contains("both populated"), "{err}");
    }

    #[test]
    fn a_file_arm_stops_the_whole_genesis() {
        let mut value = test_data_json("generated");
        value["extraConfig"]["initialFunds"] = serde_json::json!({
            "file": ["genesis", "funds.json"],
            "hash": "abcd"
        });

        let err = serde_json::from_value::<GenesisFile>(value)
            .expect_err("a genesis naming an injection file must be refused");

        assert!(err.to_string().contains("initialFunds"), "{err}");
        assert!(err.to_string().contains("genesis/funds.json"), "{err}");
    }

    #[test]
    fn injected_staking_needs_no_staking_section() {
        let mut value = test_data_json("generated");
        value
            .as_object_mut()
            .expect("the genesis must be an object")
            .remove("staking")
            .expect("the fixture must carry a staking section");

        let config: GenesisFile = serde_json::from_value(value).expect("the genesis must parse");

        let staking = config
            .staking
            .expect("injected staking must reach the accessor with no staking section");

        assert_eq!(staking.pools.expect("pools must be present").len(), 1);
        assert_eq!(staking.stake.expect("delegations must be present").len(), 1);
    }

    #[test]
    fn no_staking_anywhere_means_none() {
        let mut value = test_data_json("generated");
        let object = value
            .as_object_mut()
            .expect("the genesis must be an object");
        object.remove("staking");
        object.remove("extraConfig");

        let config: GenesisFile = serde_json::from_value(value).expect("the genesis must parse");

        assert!(
            config.staking.is_none(),
            "a genesis naming no staking anywhere must have none"
        );
    }

    #[test]
    fn an_empty_staking_section_survives() {
        let mut value = test_data_json("generated");
        value["staking"] = serde_json::json!({});
        value
            .as_object_mut()
            .expect("the genesis must be an object")
            .remove("extraConfig");

        let config: GenesisFile = serde_json::from_value(value).expect("the genesis must parse");

        let staking = config
            .staking
            .expect("an empty staking section must survive the fold");

        assert!(staking.pools.is_none(), "no pools key means no pools");
        assert!(staking.stake.is_none(), "no stake key means no stake");
    }
}
