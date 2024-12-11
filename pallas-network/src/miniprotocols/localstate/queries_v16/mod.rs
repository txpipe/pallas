// TODO: this should move to pallas::ledger crate at some point

use pallas_crypto::hash::Hash;
use std::collections::BTreeSet;
use std::hash::Hash as StdHash;
// required for derive attrs to work
use pallas_codec::minicbor::{self};

use pallas_codec::utils::{AnyUInt, Bytes, KeyValuePairs, TagWrap};
use pallas_codec::{
    minicbor::{Decode, Encode},
    utils::AnyCbor,
};

use crate::miniprotocols::Point;

use super::{Client, ClientError};

mod codec;

// https://github.com/input-output-hk/ouroboros-consensus/blob/main/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Ledger/Query.hs
#[derive(Debug, Clone, PartialEq)]
#[repr(u16)]
pub enum BlockQuery {
    GetLedgerTip,
    GetEpochNo,
    GetNonMyopicMemberRewards(AnyCbor),
    GetCurrentPParams,
    GetProposedPParamsUpdates,
    GetStakeDistribution,
    GetUTxOByAddress(Addrs),
    GetUTxOWhole,
    DebugEpochState,
    GetCBOR(Box<BlockQuery>),
    GetFilteredDelegationsAndRewardAccounts(StakeAddrs),
    GetGenesisConfig,
    DebugNewEpochState,
    DebugChainDepState,
    GetRewardProvenance,
    GetUTxOByTxIn(TxIns),
    GetStakePools,
    GetStakePoolParams(AnyCbor),
    GetRewardInfoPools,
    GetPoolState(AnyCbor),
    GetStakeSnapshots(Pools),
    GetPoolDistr(AnyCbor),
    GetStakeDelegDeposits(AnyCbor),
    GetConstitutionHash,
}

#[derive(Debug, Clone, PartialEq)]
#[repr(u16)]
pub enum HardForkQuery {
    GetInterpreter,
    GetCurrentEra,
}

pub type Epoch = u64;
pub type Proto = u16;
pub type Era = u16;

#[derive(Debug, Clone, PartialEq)]
pub enum LedgerQuery {
    BlockQuery(Era, BlockQuery),
    HardForkQuery(HardForkQuery),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    LedgerQuery(LedgerQuery),
    GetSystemStart,
    GetChainBlockNo,
    GetChainPoint,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Value {
    Coin(Coin),
    Multiasset(Coin, Multiasset<Coin>),
}

#[derive(Debug, Encode, Decode, PartialEq)]
pub struct SystemStart {
    #[n(0)]
    pub year: u32,

    #[n(1)]
    pub day_of_year: u32,

    #[n(2)]
    pub picoseconds_of_day: u64,
}

#[derive(Debug, Encode, Decode, PartialEq)]
pub struct ChainBlockNumber {
    #[n(0)]
    pub slot_timeline: u32,

    #[n(1)]
    pub block_number: u32,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RationalNumber {
    pub numerator: u64,
    pub denominator: u64,
}

pub type UnitInterval = RationalNumber;
pub type PositiveInterval = RationalNumber;

pub type ProtocolVersionMajor = u64;
pub type ProtocolVersionMinor = u64;
pub type ProtocolVersion = (ProtocolVersionMajor, ProtocolVersionMinor);

pub type CostModel = Vec<i64>;

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct CostMdls {
    #[n(0)]
    pub plutus_v1: Option<CostModel>,

    #[n(1)]
    pub plutus_v2: Option<CostModel>,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct ExUnitPrices {
    #[n(0)]
    pub mem_price: PositiveInterval,

    #[n(1)]
    pub step_price: PositiveInterval,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct ExUnits {
    #[n(0)]
    pub mem: u32,
    #[n(1)]
    pub steps: u64,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(array)]
pub struct ProtocolParam {
    #[n(0)]
    pub minfee_a: Option<u32>,
    #[n(1)]
    pub minfee_b: Option<u32>,
    #[n(2)]
    pub max_block_body_size: Option<u32>,
    #[n(3)]
    pub max_transaction_size: Option<u32>,
    #[n(4)]
    pub max_block_header_size: Option<u32>,
    #[n(5)]
    pub key_deposit: Option<Coin>,
    #[n(6)]
    pub pool_deposit: Option<Coin>,
    #[n(7)]
    pub maximum_epoch: Option<Epoch>,
    #[n(8)]
    pub desired_number_of_stake_pools: Option<u32>,
    #[n(9)]
    pub pool_pledge_influence: Option<RationalNumber>,
    #[n(10)]
    pub expansion_rate: Option<UnitInterval>,
    #[n(11)]
    pub treasury_growth_rate: Option<UnitInterval>,
    #[n(12)]
    pub protocol_version: Option<ProtocolVersion>,
    #[n(13)]
    pub min_pool_cost: Option<Coin>,
    #[n(14)]
    pub ada_per_utxo_byte: Option<Coin>,
    #[n(15)]
    pub cost_models_for_script_languages: Option<CostMdls>,
    #[n(16)]
    pub execution_costs: Option<ExUnitPrices>,
    #[n(17)]
    pub max_tx_ex_units: Option<ExUnits>,
    #[n(18)]
    pub max_block_ex_units: Option<ExUnits>,
    #[n(19)]
    pub max_value_size: Option<u32>,
    #[n(20)]
    pub collateral_percentage: Option<u32>,
    #[n(21)]
    pub max_collateral_inputs: Option<u32>,
}

#[derive(Debug, Encode, Decode, PartialEq)]
pub struct StakeDistribution {
    #[n(0)]
    pub pools: KeyValuePairs<Bytes, Pool>,
}

#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct Pool {
    #[n(0)]
    pub stakes: RationalNumber,

    #[n(1)]
    pub hashes: Bytes,
}

/// Type used at [GenesisConfig], which is a fraction that is CBOR-encoded
/// as an untagged array.
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct Fraction {
    #[n(0)]
    pub num: u64,

    #[n(1)]
    pub den: u64,
}

pub type Addr = Bytes;

pub type Addrs = Vec<Addr>;

#[derive(Debug, Encode, Decode, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct StakeAddr {
    #[n(0)]
    addr_type: u8,
    #[n(1)]
    payload: Addr,
}

impl From<(u8, Bytes)> for StakeAddr {
    fn from((addr_type, payload): (u8, Bytes)) -> Self {
        Self { addr_type, payload }
    }
}

pub type StakeAddrs = BTreeSet<StakeAddr>;
pub type Delegations = KeyValuePairs<StakeAddr, Bytes>;
pub type RewardAccounts = KeyValuePairs<StakeAddr, u64>;

#[derive(Debug, PartialEq, Clone)]
pub struct FilteredDelegsRewards {
    pub delegs: Delegations,
    pub rewards: RewardAccounts,
}

pub type Pools = BTreeSet<Bytes>;

pub type Coin = AnyUInt;

pub type PolicyId = Hash<28>;

pub type AssetName = Bytes;

pub type Multiasset<A> = KeyValuePairs<PolicyId, KeyValuePairs<AssetName, A>>;

#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct UTxOByAddress {
    #[n(0)]
    pub utxo: KeyValuePairs<UTxO, TransactionOutput>,
}

pub type UTxOByTxin = UTxOByAddress;

pub type UTxOWhole = UTxOByAddress;

// Bytes CDDL ->  #6.121([ * #6.121([ *datum ]) ])
pub type Datum = (Era, TagWrap<Bytes, 24>);

// From `pallas-primitives`, with fewer `derive`s
#[derive(Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct TransactionInput {
    #[n(0)]
    pub transaction_id: Hash<32>,

    #[n(1)]
    pub index: u64,
}

pub type TxIns = BTreeSet<TransactionInput>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TransactionOutput {
    Current(PostAlonsoTransactionOutput),
    Legacy(LegacyTransactionOutput),
}

#[derive(Debug, Encode, Decode, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct PostAlonsoTransactionOutput {
    #[n(0)]
    pub address: Bytes,

    #[n(1)]
    pub amount: Value,

    #[n(2)]
    pub inline_datum: Option<Datum>,

    #[n(3)]
    pub script_ref: Option<TagWrap<Bytes, 24>>,
}

#[derive(Debug, Encode, Decode, PartialEq, Eq, Clone)]
pub struct LegacyTransactionOutput {
    #[n(0)]
    pub address: Bytes,

    #[n(1)]
    pub amount: Value,

    #[n(2)]
    pub datum_hash: Option<Hash<32>>,
}

#[derive(Debug, Encode, Decode, PartialEq, Clone, StdHash, Eq)]
pub struct UTxO {
    #[n(0)]
    pub transaction_id: Hash<32>,

    #[n(1)]
    pub index: AnyUInt,
}

#[derive(Debug, Encode, Decode, PartialEq)]
pub struct StakeSnapshot {
    #[n(0)]
    pub snapshots: Snapshots,
}

#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct Snapshots {
    #[n(0)]
    pub stake_snapshots: KeyValuePairs<Bytes, Stakes>,

    #[n(1)]
    pub snapshot_stake_mark_total: u64,

    #[n(2)]
    pub snapshot_stake_set_total: u64,

    #[n(3)]
    pub snapshot_stake_go_total: u64,
}

#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct Stakes {
    #[n(0)]
    pub snapshot_mark_pool: u64,

    #[n(1)]
    pub snapshot_set_pool: u64,

    #[n(2)]
    pub snapshot_go_pool: u64,
}

#[derive(Debug, Encode, Decode, PartialEq)]
pub struct GenesisConfig {
    #[n(0)]
    pub system_start: SystemStart,

    #[n(1)]
    pub network_magic: u32,

    #[n(2)]
    pub network_id: u32,

    #[n(3)]
    pub active_slots_coefficient: Fraction,

    #[n(4)]
    pub security_param: u32,

    #[n(5)]
    pub epoch_length: u32,

    #[n(6)]
    pub slots_per_kes_period: u32,

    #[n(7)]
    pub max_kes_evolutions: u32,

    #[n(8)]
    pub slot_length: u32,

    #[n(9)]
    pub update_quorum: u32,

    #[n(10)]
    pub max_lovelace_supply: Coin,
}

/// Get the current tip of the ledger.
pub async fn get_chain_point(client: &mut Client) -> Result<Point, ClientError> {
    let query = Request::GetChainPoint;
    let result = client.query(query).await?;

    Ok(result)
}

/// Get the current era.
pub async fn get_current_era(client: &mut Client) -> Result<Era, ClientError> {
    let query = HardForkQuery::GetCurrentEra;
    let query = LedgerQuery::HardForkQuery(query);
    let query = Request::LedgerQuery(query);
    let result = client.query(query).await?;

    Ok(result)
}

/// Get the system start time.
pub async fn get_system_start(client: &mut Client) -> Result<SystemStart, ClientError> {
    let query = Request::GetSystemStart;
    let result = client.query(query).await?;

    Ok(result)
}

/// Get the block number for the current tip.
pub async fn get_chain_block_no(client: &mut Client) -> Result<ChainBlockNumber, ClientError> {
    let query = Request::GetChainBlockNo;
    let result = client.query(query).await?;

    Ok(result)
}

/// Get the current protocol parameters.
pub async fn get_current_pparams(
    client: &mut Client,
    era: u16,
) -> Result<Vec<ProtocolParam>, ClientError> {
    let query = BlockQuery::GetCurrentPParams;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result = client.query(query).await?;

    Ok(result)
}

/// Get the block number for the current tip.
pub async fn get_block_epoch_number(client: &mut Client, era: u16) -> Result<u32, ClientError> {
    let query = BlockQuery::GetEpochNo;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,): (_,) = client.query(query).await?;

    Ok(result)
}

/// Get the current stake distribution for the given era.
pub async fn get_stake_distribution(
    client: &mut Client,
    era: u16,
) -> Result<StakeDistribution, ClientError> {
    let query = BlockQuery::GetStakeDistribution;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result = client.query(query).await?;

    Ok(result)
}

/// Get the UTxO set for the given era.
pub async fn get_utxo_by_address(
    client: &mut Client,
    era: u16,
    addrs: Addrs,
) -> Result<UTxOByAddress, ClientError> {
    let query = BlockQuery::GetUTxOByAddress(addrs);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result = client.query(query).await?;

    Ok(result)
}

/// Get stake snapshots for the given era and stake pools.
/// If `pools` are empty, all pools are queried.
/// Otherwise, only the specified pool is queried.
/// Note: This Query is limited by 1 pool per request.
pub async fn get_stake_snapshots(
    client: &mut Client,
    era: u16,
    pools: BTreeSet<Bytes>,
) -> Result<StakeSnapshot, ClientError> {
    let query = BlockQuery::GetStakeSnapshots(pools);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result = client.query(query).await?;

    Ok(result)
}

pub async fn get_cbor(
    client: &mut Client,
    era: u16,
    query: BlockQuery,
) -> Result<Vec<TagWrap<Bytes, 24>>, ClientError> {
    let query = BlockQuery::GetCBOR(Box::new(query));
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result = client.query(query).await?;

    Ok(result)
}

/// Get the genesis configuration for the given era.
pub async fn get_genesis_config(
    client: &mut Client,
    era: u16,
) -> Result<Vec<GenesisConfig>, ClientError> {
    let query = BlockQuery::GetGenesisConfig;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result = client.query(query).await?;

    Ok(result)
}

/// Get the delegations and rewards for the given stake addresses.
pub async fn get_filtered_delegations_rewards(
    client: &mut Client,
    era: u16,
    addrs: StakeAddrs,
) -> Result<FilteredDelegsRewards, ClientError> {
    let query = BlockQuery::GetFilteredDelegationsAndRewardAccounts(addrs);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result = client.query(query).await?;

    Ok(result)
}

/// Get a subset of the UTxO, filtered by transaction input.
pub async fn get_utxo_by_txin(
    client: &mut Client,
    era: u16,
    txins: TxIns,
) -> Result<UTxOByTxin, ClientError> {
    let query = BlockQuery::GetUTxOByTxIn(txins);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result = client.query(query).await?;

    Ok(result)
}

/// Get the /entire/ UTxO.
pub async fn get_utxo_whole(
    client: &mut Client,
    era: u16,
) -> Result<UTxOWhole, ClientError> {
    let query = BlockQuery::GetUTxOWhole;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result = client.query(query).await?;

    Ok(result)
}
