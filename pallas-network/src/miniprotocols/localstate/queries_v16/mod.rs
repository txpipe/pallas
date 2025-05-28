// TODO: this should move to pallas::ledger crate at some point

use pallas_crypto::hash::Hash;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::Hash as StdHash;
use std::ops::Deref;
// required for derive attrs to work
use pallas_codec::minicbor::{self};

use pallas_codec::minicbor::{Decode, Encode};
use pallas_codec::utils::{
    AnyCbor, AnyUInt, Bytes, CborWrap, Int, KeyValuePairs, MaybeIndefArray, NonEmptyKeyValuePairs,
    Nullable, TagWrap,
};

pub mod primitives;

pub use primitives::{PoolMetadata, Relay};

use crate::miniprotocols::localtxsubmission::primitives::{
    CommitteeColdCredential, CommitteeHotCredential, ScriptRef, StakeCredential,
};
use crate::miniprotocols::Point;

use crate::miniprotocols::localtxsubmission::{Network, SMaybe};

use super::{Client, ClientError};

mod codec;

// https://github.com/input-output-hk/ouroboros-consensus/blob/main/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Ledger/Query.hs
#[derive(Debug, Clone, PartialEq)]
#[repr(u16)]
pub enum BlockQuery {
    GetLedgerTip,
    GetEpochNo,
    GetNonMyopicMemberRewards(TaggedSet<Either<Coin, StakeAddr>>),
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
    GetStakePoolParams(Pools),
    GetRewardInfoPools,
    GetPoolState(SMaybe<Pools>),
    GetStakeSnapshots(SMaybe<Pools>),
    GetPoolDistr(SMaybe<Pools>),
    GetStakeDelegDeposits(TaggedSet<StakeAddr>),
    GetConstitution,
    GetGovState,
    GetDRepState(TaggedSet<Credential>),
    GetDRepStakeDistr(TaggedSet<DRep>),
    GetCommitteeMembersState(
        TaggedSet<Credential>,
        TaggedSet<Credential>,
        TaggedSet<MemberStatus>,
    ),
    GetFilteredVoteDelegatees(StakeAddrs),
    GetAccountState,
    GetSPOStakeDistr(Pools),
    GetProposals(TaggedSet<GovActionId>),
    GetRatifyState,
    GetFuturePParams,
    GetBigLedgerPeerSnapshot,
}

pub type Credential = StakeAddr;

/// Updates to the protocol params as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/Core/PParams.hs#L151)
/// (via [`EraPParams`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/Core/PParams.hs#L255-L258)).
#[derive(Encode, Decode, Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
#[cbor(map)]
pub struct PParamsUpdate {
    #[n(0)]
    pub minfee_a: Option<u64>,
    #[n(1)]
    pub minfee_b: Option<u64>,
    #[n(2)]
    pub max_block_body_size: Option<u64>,
    #[n(3)]
    pub max_transaction_size: Option<u64>,
    #[n(4)]
    pub max_block_header_size: Option<u64>,
    #[n(5)]
    pub key_deposit: Option<Coin>,
    #[n(6)]
    pub pool_deposit: Option<Coin>,
    #[n(7)]
    pub maximum_epoch: Option<Epoch>,
    #[n(8)]
    pub desired_number_of_stake_pools: Option<u64>,
    #[n(9)]
    pub pool_pledge_influence: Option<RationalNumber>,
    #[n(10)]
    pub expansion_rate: Option<UnitInterval>,
    #[n(11)]
    pub treasury_growth_rate: Option<UnitInterval>,

    #[n(16)]
    pub min_pool_cost: Option<Coin>,
    #[n(17)]
    pub ada_per_utxo_byte: Option<Coin>,
    #[n(18)]
    pub cost_models_for_script_languages: Option<CostModels>,
    #[n(19)]
    pub execution_costs: Option<ExUnitPrices>,
    #[n(20)]
    pub max_tx_ex_units: Option<ExUnits>,
    #[n(21)]
    pub max_block_ex_units: Option<ExUnits>,
    #[n(22)]
    pub max_value_size: Option<u64>,
    #[n(23)]
    pub collateral_percentage: Option<u64>,
    #[n(24)]
    pub max_collateral_inputs: Option<u64>,

    #[n(25)]
    pub pool_voting_thresholds: Option<PoolVotingThresholds>,
    #[n(26)]
    pub drep_voting_thresholds: Option<DRepVotingThresholds>,
    #[n(27)]
    pub min_committee_size: Option<u64>,
    #[n(28)]
    pub committee_term_limit: Option<Epoch>,
    #[n(29)]
    pub governance_action_validity_period: Option<Epoch>,
    #[n(30)]
    pub governance_action_deposit: Option<Coin>,
    #[n(31)]
    pub drep_deposit: Option<Coin>,
    #[n(32)]
    pub drep_inactivity_period: Option<Epoch>,
    #[n(33)]
    pub minfee_refscript_cost_per_byte: Option<UnitInterval>,
}

/// Propoped updates to the protocol params as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/shelley/impl/src/Cardano/Ledger/Shelley/PParams.hs#L510-L511).
pub type ProposedPPUpdates = BTreeMap<Bytes, PParamsUpdate>;

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct AccountState {
    #[n(0)]
    pub treasury: Coin,
    #[n(1)]
    pub reserves: Coin,
}

/// Committee authorization as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/CertState.hs#L294-L298).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CommitteeAuthorization {
    HotCredential(Credential),
    MemberResigned(SMaybe<Anchor>),
}

/// Committee authorization as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L532-L537).
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Committee {
    #[n(0)]
    pub members: BTreeMap<Credential, Epoch>,
    #[n(1)]
    pub threshold: UnitInterval,
}

/// Hot credential auth status as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-api/src/Cardano/Ledger/Api/State/Query/CommitteeMembersState.hs#L55-L60).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum HotCredAuthStatus {
    MemberAuthorized(Credential),
    MemberNotAuthorized,
    MemberResigned(SMaybe<Anchor>),
}

/// Next epoch change status as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-api/src/Cardano/Ledger/Api/State/Query/CommitteeMembersState.hs#L77-L84).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NextEpochChange {
    ToBeEnacted,
    ToBeRemoved,
    NoChangeExpected,
    ToBeExpired,
    TermAdjusted(Epoch),
}

/// Member status as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-api/src/Cardano/Ledger/Api/State/Query/CommitteeMembersState.hs#L39-L46).
#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cbor(index_only)]
pub enum MemberStatus {
    #[n(0)]
    Active,
    #[n(1)]
    Expired,
    #[n(2)]
    Unrecognized,
}

/// Committee member state as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-api/src/Cardano/Ledger/Api/State/Query/CommitteeMembersState.hs#L106-L113). Not to be confused with plural [CommitteeMembersState].
#[derive(Debug, Encode, Decode, PartialEq, Eq, Clone)]
pub struct CommitteeMemberState {
    #[n(0)]
    pub hot_cred_auth_status: HotCredAuthStatus,
    #[n(1)]
    pub status: MemberStatus,
    #[n(2)]
    pub expiration: SMaybe<Epoch>,
    #[n(3)]
    pub next_epoch_change: NextEpochChange,
}

/// Committee members' state as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-api/src/Cardano/Ledger/Api/State/Query/CommitteeMembersState.hs#L149-L154). Not to be confused with singular [CommitteeMemberState].
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct CommitteeMembersState {
    #[n(0)]
    pub committee: BTreeMap<Credential, CommitteeMemberState>,
    #[n(1)]
    pub threshold: SMaybe<RationalNumber>,
    #[n(2)]
    pub epoch: Epoch,
}

/// DRep thresholds as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/DRep.hs#L52-L57
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum DRep {
    KeyHash(Bytes),
    ScriptHash(Bytes),
    AlwaysAbstain,
    AlwaysNoConfidence,
}

/// Governance action id as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L167-L170),
/// via [Transaction ID](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/TxIn.hs#L56
/// and [Action Index](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L154).
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct GovActionId {
    #[n(0)]
    pub tx_id: Hash<32>,
    #[n(1)]
    pub gov_action_ix: u32,
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
    pub year: BigInt,

    #[n(1)]
    pub day_of_year: i64,

    #[n(2)]
    pub picoseconds_of_day: BigInt,
}

#[derive(Debug, Encode, Decode, PartialEq)]
pub struct ChainBlockNumber {
    #[n(0)]
    pub slot_timeline: u32,

    #[n(1)]
    pub block_number: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
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

#[derive(Encode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
#[cbor(map)]
pub struct CostModels {
    #[n(0)]
    pub plutus_v1: Option<CostModel>,

    #[n(1)]
    pub plutus_v2: Option<CostModel>,

    #[n(2)]
    pub plutus_v3: Option<CostModel>,

    #[cbor(skip)]
    pub unknown: KeyValuePairs<u64, CostModel>,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct ExUnitPrices {
    #[n(0)]
    pub mem_price: PositiveInterval,

    #[n(1)]
    pub step_price: PositiveInterval,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct ExUnits {
    #[n(0)]
    pub mem: u64,
    #[n(1)]
    pub steps: u64,
}
/// Pool voting thresholds as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/PParams.hs#L223-L229).
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct PoolVotingThresholds {
    #[n(0)]
    pub motion_no_confidence: UnitInterval,
    #[n(1)]
    pub committee_normal: UnitInterval,
    #[n(2)]
    pub committee_no_confidence: UnitInterval,
    #[n(3)]
    pub hard_fork_initiation: UnitInterval,
    #[n(4)]
    pub pp_security_group: UnitInterval,
}

/// DRrep voting thresholds as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/PParams.hs#L295-L306).
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct DRepVotingThresholds {
    #[n(0)]
    pub motion_no_confidence: UnitInterval,
    #[n(1)]
    pub committee_normal: UnitInterval,
    #[n(2)]
    pub committee_no_confidence: UnitInterval,
    #[n(3)]
    pub update_to_constitution: UnitInterval,
    #[n(4)]
    pub hard_fork_initiation: UnitInterval,
    #[n(5)]
    pub pp_network_group: UnitInterval,
    #[n(6)]
    pub pp_economic_group: UnitInterval,
    #[n(7)]
    pub pp_technical_group: UnitInterval,
    #[n(8)]
    pub pp_gov_group: UnitInterval,
    #[n(9)]
    pub treasury_withdrawal: UnitInterval,
}

/// Conway era protocol parameters, corresponding to [`ConwayPParams`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/PParams.hs#L512-L579)
/// in the Haskell sources.
/// @todo: Encoding should be handled manually, Encode derive won't be correct.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
#[cbor(map)]
pub struct ProtocolParam {
    #[n(0)]
    pub minfee_a: Option<u64>,
    #[n(1)]
    pub minfee_b: Option<u64>,
    #[n(2)]
    pub max_block_body_size: Option<u64>,
    #[n(3)]
    pub max_transaction_size: Option<u64>,
    #[n(4)]
    pub max_block_header_size: Option<u64>,
    #[n(5)]
    pub key_deposit: Option<Coin>,
    #[n(6)]
    pub pool_deposit: Option<Coin>,
    #[n(7)]
    pub maximum_epoch: Option<Epoch>,
    #[n(8)]
    pub desired_number_of_stake_pools: Option<u64>,
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
    pub cost_models_for_script_languages: Option<CostModels>,
    #[n(16)]
    pub execution_costs: Option<ExUnitPrices>,
    #[n(17)]
    pub max_tx_ex_units: Option<ExUnits>,
    #[n(18)]
    pub max_block_ex_units: Option<ExUnits>,
    #[n(19)]
    pub max_value_size: Option<u64>,
    #[n(20)]
    pub collateral_percentage: Option<u64>,
    #[n(21)]
    pub max_collateral_inputs: Option<u64>,
    #[n(22)]
    pub pool_voting_thresholds: Option<PoolVotingThresholds>,
    #[n(23)]
    pub drep_voting_thresholds: Option<DRepVotingThresholds>,
    #[n(24)]
    pub min_committee_size: Option<u64>,
    #[n(25)]
    pub committee_term_limit: Option<Epoch>,
    #[n(26)]
    pub governance_action_validity_period: Option<Epoch>,
    #[n(27)]
    pub governance_action_deposit: Option<Coin>,
    #[n(28)]
    pub drep_deposit: Option<Coin>,
    #[n(29)]
    pub drep_inactivity_period: Option<Epoch>,
    #[n(30)]
    pub minfee_refscript_cost_per_byte: Option<UnitInterval>,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct CurrentProtocolParam {
    #[n(0)]
    pub minfee_a: Option<u64>,
    #[n(1)]
    pub minfee_b: Option<u64>,
    #[n(2)]
    pub max_block_body_size: Option<u64>,
    #[n(3)]
    pub max_transaction_size: Option<u64>,
    #[n(4)]
    pub max_block_header_size: Option<u64>,
    #[n(5)]
    pub key_deposit: Option<Coin>,
    #[n(6)]
    pub pool_deposit: Option<Coin>,
    #[n(7)]
    pub maximum_epoch: Option<Epoch>,
    #[n(8)]
    pub desired_number_of_stake_pools: Option<u64>,
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
    pub cost_models_for_script_languages: Option<CostModels>,
    #[n(16)]
    pub execution_costs: Option<ExUnitPrices>,
    #[n(17)]
    pub max_tx_ex_units: Option<ExUnits>,
    #[n(18)]
    pub max_block_ex_units: Option<ExUnits>,
    #[n(19)]
    pub max_value_size: Option<u64>,
    #[n(20)]
    pub collateral_percentage: Option<u64>,
    #[n(21)]
    pub max_collateral_inputs: Option<u64>,
    #[n(22)]
    pub pool_voting_thresholds: Option<PoolVotingThresholds>,
    #[n(23)]
    pub drep_voting_thresholds: Option<DRepVotingThresholds>,
    #[n(24)]
    pub min_committee_size: Option<u64>,
    #[n(25)]
    pub committee_term_limit: Option<Epoch>,
    #[n(26)]
    pub governance_action_validity_period: Option<Epoch>,
    #[n(27)]
    pub governance_action_deposit: Option<Coin>,
    #[n(28)]
    pub drep_deposit: Option<Coin>,
    #[n(29)]
    pub drep_inactivity_period: Option<Epoch>,
    #[n(30)]
    pub minfee_refscript_cost_per_byte: Option<UnitInterval>,
}

pub type StakeDistribution = KeyValuePairs<Bytes, Pool>;

/// Tuple struct based on `BTreeSet` which uses the "Set" CBOR tag.
pub type TaggedSet<T> = TagWrap<BTreeSet<T>, 258>;

#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct Pool {
    #[n(0)]
    pub stakes: RationalNumber,

    #[n(1)]
    pub hashes: Bytes,
}

// Essentially the `PoolRegistration` component of `Certificate` at
// `pallas-primitives/src/alonzo/model.rs`, with types modified for the present
// context
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct PoolParams {
    #[n(0)]
    pub operator: Bytes,

    #[n(1)]
    pub vrf_keyhash: Bytes,

    #[n(2)]
    pub pledge: Coin,

    #[n(3)]
    pub cost: Coin,

    #[n(4)]
    pub margin: UnitInterval,

    #[n(5)]
    pub reward_account: Addr,

    #[n(6)]
    pub pool_owners: Pools,

    #[n(7)]
    pub relays: Vec<Relay>,

    #[n(8)]
    pub pool_metadata: Nullable<PoolMetadata>,
}

/// State of Pools at the Cardano ledger, corresponding to [`PState`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/CertState.hs#L246-L259)
/// in the Haskell sources.
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct PState {
    #[n(0)]
    stake_pool_params: BTreeMap<Bytes, PoolParams>,
    #[n(1)]
    future_stake_pool_params: BTreeMap<Bytes, PoolParams>,
    #[n(2)]
    retiring: BTreeMap<Bytes, u32>,
    #[n(3)]
    deposits: BTreeMap<Bytes, Coin>,
}

/// Stake controlled by a single pool, corresponding to [`IndividualPoolStake`](https://github.com/IntersectMBO/ouroboros-consensus/blob/358305b09f8fa1a85f076b20a51b4af03e827071/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Ledger/Query/Types.hs#L32-L35)
/// in the Haskell sources (not to be confused with [the `cardano-ledger` notion with the same name](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/PoolDistr.hs#L53-L61)).
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct IndividualPoolStake {
    #[n(0)]
    individual_pool_stake: RationalNumber,
    #[n(1)]
    individual_pool_stake_vrf: Bytes,
}

/// Map from pool hashes to [IndividualPoolStake]s, corresponding to [`PoolDistr`](https://github.com/IntersectMBO/ouroboros-consensus/blob/358305b09f8fa1a85f076b20a51b4af03e827071/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Ledger/Query/Types.hs#L62-L64)
/// in the Haskell sources (not to be confused with [the `cardano-ledger` notion with the same name](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/PoolDistr.hs#L100-L106)).
pub type PoolDistr = BTreeMap<Bytes, IndividualPoolStake>;

/// Anchor as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L867-L870).
#[derive(Debug, Encode, Decode, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct Anchor {
    #[n(0)]
    pub url: String,
    #[n(1)]
    pub data_hash: Bytes,
}

/// Constitution as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L884-L887).
#[derive(Debug, Encode, Decode, Eq, PartialEq, Clone, PartialOrd, Ord)]
pub struct Constitution {
    #[n(0)]
    pub anchor: Anchor,
    #[n(1)]
    pub script: Option<ScriptHash>,
}

/// TODO: Votes as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L365-L368).
#[derive(Debug, Decode, Encode, PartialEq, Eq, Clone)]
#[cbor(index_only)]
pub enum Vote {
    #[n(0)]
    No,
    #[n(1)]
    Yes,
    #[n(2)]
    Abstain,
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct FieldedRewardAccount {
    pub network: Network,
    pub stake_credential: StakeCredential,
}

impl From<&[u8]> for FieldedRewardAccount {
    fn from(bytes: &[u8]) -> Self {
        let network = if bytes[0] & 0b00000001 != 0 {
            Network::Mainnet
        } else {
            Network::Testnet
        };

        let mut hash = [0; 28];
        hash.copy_from_slice(&bytes[1..29]);
        let stake_credential = if &bytes[0] & 0b00010000 != 0 {
            StakeCredential::ScriptHash(hash.into())
        } else {
            StakeCredential::AddrKeyhash(hash.into())
        };

        FieldedRewardAccount {
            network,
            stake_credential,
        }
    }
}

pub type ScriptHash = Hash<28>;

/// Governance action as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L785-L824).
#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord)]
#[allow(clippy::large_enum_variant)]
pub enum GovAction {
    ParameterChange(Option<GovPurposeId>, PParamsUpdate, Option<ScriptHash>),
    HardForkInitiation(Option<GovPurposeId>, ProtocolVersion),
    TreasuryWithdrawals(
        KeyValuePairs<FieldedRewardAccount, Coin>,
        Option<ScriptHash>,
    ),
    NoConfidence(Option<GovPurposeId>),
    UpdateCommittee(
        Option<GovPurposeId>,
        TaggedSet<CommitteeColdCredential>,
        BTreeMap<CommitteeHotCredential, Epoch>,
        UnitInterval,
    ),
    NewConstitution(Option<GovPurposeId>, Constitution),
    InfoAction,
}

/// Proposal procedure state as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L476-L481
#[derive(Debug, Encode, Decode, Eq, PartialEq, Clone, PartialOrd, Ord)]
pub struct ProposalProcedure {
    #[n(0)]
    pub deposit: Coin,
    #[n(1)]
    pub return_addr: FieldedRewardAccount,
    #[n(2)]
    pub gov_action: GovAction,
    #[n(3)]
    pub anchor: Anchor,
}

/// Governance action state as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L211-L219).
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct GovActionState {
    #[n(0)]
    pub id: GovActionId,
    #[n(1)]
    pub committee_votes: BTreeMap<Credential, Vote>,
    #[n(2)]
    pub drep_votes: BTreeMap<Credential, Vote>,
    #[n(3)]
    pub stake_pool_votes: BTreeMap<Bytes, Vote>,
    #[n(4)]
    pub proposal_procedure: ProposalProcedure,
    #[n(5)]
    pub proposed_in: Epoch,
    #[n(6)]
    pub expires_after: Epoch,
}

/// Governance relation as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L636-L641)
/// (where the higher order argument `f` is `StrictMaybe`).
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct GovRelation {
    #[n(0)]
    pub pparam_update: SMaybe<GovPurposeId>,
    #[n(1)]
    pub hard_fork: SMaybe<GovPurposeId>,
    #[n(2)]
    pub committee: SMaybe<GovPurposeId>,
    #[n(3)]
    pub constitution: SMaybe<GovPurposeId>,
}

/// TODO: Ledger peer snapshot as defined [in the Haskell sources](https://github.com/IntersectMBO/ouroboros-network/blob/df3431f95ef9e47a8a26fd3376efd61ed0837747/ouroboros-network-api/src/Ouroboros/Network/PeerSelection/LedgerPeers/Type.hs#L51-L53).
pub type LedgerPeerSnapshot = AnyCbor;

/// Governance purpose Id as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L618-L620).
pub type GovPurposeId = GovActionId;

/// Enact state as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Internal.hs#L146-L157).
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct EnactState {
    #[n(0)]
    pub committee: SMaybe<Committee>,
    #[n(1)]
    pub constitution: Constitution,
    #[n(2)]
    pub cur_pparams: ProtocolParam,
    #[n(3)]
    pub prev_pparams: ProtocolParam,
    #[n(4)]
    pub treasury: Coin,
    #[n(5)]
    pub withdrawals: BTreeMap<Credential, Coin>,
    #[n(6)]
    pub prev_gov_action_ids: GovRelation,
}

/// Ratify state as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Internal.hs#L269-L275).
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct RatifyState {
    #[n(0)]
    pub enact_state: EnactState,
    #[n(1)]
    pub enacted: Vec<GovActionState>,
    #[n(2)]
    pub expired: TaggedSet<GovActionId>,
    #[n(3)]
    pub delayed: bool,
}

pub type Proposals = AnyCbor;
pub type DRepPulsingState = AnyCbor;

/// Future protocol parameters as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/shelley/impl/src/Cardano/Ledger/Shelley/Governance.hs#L137-L148).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FuturePParams {
    NoPParamsUpdate,
    DefinitePParamsUpdate(PParamsUpdate),
    PotentialPParamsUpdate(SMaybe<PParamsUpdate>),
}

/// Governance state as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance.hs#L241-L256)
/// (via [`EraGov`](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/shelley/impl/src/Cardano/Ledger/Shelley/Governance.hs#L83-L85) and
/// [this instance](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance.hs#L388-L389)).
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct GovState {
    #[n(0)]
    pub proposals: Proposals,
    #[n(1)]
    pub committee: SMaybe<Committee>,
    #[n(2)]
    pub constitution: Constitution,
    #[n(3)]
    pub cur_pparams: ProtocolParam,
    #[n(4)]
    pub prev_pparams: ProtocolParam,
    #[n(5)]
    pub future_pparams: FuturePParams,
    #[n(6)]
    pub drep_pulsing_state: DRepPulsingState,
}

/// DRep state as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/DRep.hs#L125-L130).
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct DRepState {
    #[n(0)]
    pub expiry: Epoch,
    #[n(1)]
    pub anchor: SMaybe<Anchor>,
    #[n(2)]
    pub deposit: Coin,
    #[n(3)]
    pub delegs: TaggedSet<StakeAddr>,
}

/// Flat encoding coincides with the one at the [Cardano ledger](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-binary/src/Cardano/Ledger/Binary/Encoding/EncCBOR.hs#L767-L769).
#[derive(Debug, PartialEq, Clone, Eq, PartialOrd, Ord)]
pub enum Either<S, T> {
    Left(S),
    Right(T),
}

/// Map corresponding to [the type with the same name](https://github.com/IntersectMBO/ouroboros-consensus/blob/e924f61d1fe63d25e9ecd8499b705aff4d553209/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Ledger/Query.hs#L103-L107)
/// in the Haskell sources.
pub type NonMyopicMemberRewards = BTreeMap<Either<Coin, StakeAddr>, BTreeMap<Bytes, Coin>>;

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

#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct FilteredDelegsRewards {
    #[n(0)]
    pub delegs: Delegations,
    #[n(1)]
    pub rewards: RewardAccounts,
}

/// Set of pool hashes.
///
/// The use of `BTreeMap`s (as per `TaggedSet` definition) ensures that the
/// hashes are in order (otherwise, the node will reject some queries).
pub type Pools = TaggedSet<Bytes>;

pub type Coin = AnyUInt;

pub type PolicyId = Hash<28>;

pub type AssetName = Bytes;

pub type Multiasset<A> = NonEmptyKeyValuePairs<PolicyId, NonEmptyKeyValuePairs<AssetName, A>>;

pub type UTxOByAddress = KeyValuePairs<UTxO, TransactionOutput>;

pub type UTxOByTxin = UTxOByAddress;

pub type UTxOWhole = UTxOByAddress;

// Bytes CDDL ->  #6.121([ * #6.121([ *datum ]) ])
pub type Datum = (Era, TagWrap<Bytes, 24>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DatumOption {
    Hash(DatumHash),
    Data(CborWrap<PlutusData>),
}

pub type DatumHash = Hash<32>;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum PlutusData {
    Constr(Constr<PlutusData>),
    Map(KeyValuePairs<PlutusData, PlutusData>),
    BigInt(BigInt),
    BoundedBytes(BoundedBytes),
    Array(MaybeIndefArray<PlutusData>),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum BigInt {
    Int(Int),
    BigUInt(BoundedBytes),
    BigNInt(BoundedBytes),
}

impl From<Int> for BigInt {
    fn from(value: Int) -> Self {
        Self::Int(value)
    }
}

impl From<i32> for BigInt {
    fn from(x: i32) -> Self {
        Self::Int(Int::from(x))
    }
}

impl From<i64> for BigInt {
    fn from(x: i64) -> Self {
        Self::Int(Int::from(x))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoundedBytes(Vec<u8>);

impl From<Vec<u8>> for BoundedBytes {
    fn from(xs: Vec<u8>) -> Self {
        BoundedBytes(xs)
    }
}

impl From<BoundedBytes> for Vec<u8> {
    fn from(b: BoundedBytes) -> Self {
        b.0
    }
}

impl Deref for BoundedBytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<String> for BoundedBytes {
    type Error = hex::FromHexError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let v = hex::decode(value)?;
        Ok(BoundedBytes(v))
    }
}

impl From<BoundedBytes> for String {
    fn from(b: BoundedBytes) -> Self {
        hex::encode(b.deref())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Constr<A> {
    pub tag: u64,
    pub any_constructor: Option<u64>,
    pub fields: MaybeIndefArray<A>,
}
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
    pub inline_datum: Option<DatumOption>,

    #[n(3)]
    pub script_ref: Option<CborWrap<ScriptRef>>,
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

#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct StakeSnapshots {
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

/// Macro to generate an async function with specific parameters and logic.
macro_rules! block_query_with_args {
    (
        $(#[doc = $doc:expr])*
        $fn_name:ident,
        $variant:ident,
        ( $( $arg_name:ident : $arg_type:ty ),* ),
        $type2:ty,
    ) => {
        $(#[doc = $doc])*
        pub async fn $fn_name(
            client: &mut Client,
            era: u16,
            $( $arg_name : $arg_type ),*,
        ) -> Result<$type2, ClientError> {
            let query = BlockQuery::$variant($( $arg_name ),*);
            let query = LedgerQuery::BlockQuery(era, query);
            let query = Request::LedgerQuery(query);
            let (result,) = client.query(query).await?;

            Ok(result)
        }
    };
}

block_query_with_args! {
    #[doc = "Get stake snapshots for the given era and stake pools."]
    get_stake_snapshots,
    GetStakeSnapshots,
    (val : SMaybe<Pools>),
    StakeSnapshots,
}

block_query_with_args! {
    #[doc = "Get the UTxO set for the given era."]
    get_utxo_by_address,
    GetUTxOByAddress,
    (val : Addrs),
    UTxOByAddress,
}

block_query_with_args! {
    #[doc = "Get parameters for the given pools."]
    get_stake_pool_params,
    GetStakePoolParams,
    (val : Pools),
    BTreeMap<Bytes, PoolParams>,
}

block_query_with_args! {
    #[doc = "Get the current state of the given pools, or of all of them in case of a `SMaybe::None`."]
    get_pool_state,
    GetPoolState,
    (val : SMaybe<Pools>),
    PState,
}

block_query_with_args! {
    #[doc = "Get the stake controlled the given pools, or of all of them in case of a `SMaybe::None`."]
    get_pool_distr,
    GetPoolDistr,
    (val : SMaybe<Pools>),
    PoolDistr,
}

block_query_with_args! {
    get_non_myopic_member_rewards,
    GetNonMyopicMemberRewards,
    (val : TaggedSet<Either<Coin, StakeAddr>>),
    NonMyopicMemberRewards,
}

block_query_with_args! {
    #[doc = "Get the delegations and rewards for the given stake addresses."]
    get_filtered_delegations_rewards,
    GetFilteredDelegationsAndRewardAccounts,
    (val : StakeAddrs),
    FilteredDelegsRewards,
}

block_query_with_args! {
    #[doc = "Get a subset of the UTxO, filtered by transaction input."]
    get_utxo_by_txin,
    GetUTxOByTxIn,
    (val : TxIns),
    UTxOByTxin,
}

block_query_with_args! {
    #[doc = "Get the key deposits from each stake credential given."]
    get_stake_deleg_deposits,
    GetStakeDelegDeposits,
    (val : TaggedSet<StakeAddr>),
    BTreeMap<StakeAddr, Coin>,
}

block_query_with_args! {
    #[doc = "Get the current DRep state."]
    get_drep_state,
    GetDRepState,
    (val : TaggedSet<StakeAddr>),
    BTreeMap<StakeAddr, DRepState>,
}

block_query_with_args! {
    #[doc = "Get the current DRep stake distribution."]
    get_drep_stake_distr,
    GetDRepStakeDistr,
    (val : TaggedSet<DRep>),
    BTreeMap<DRep, Coin>,
}

block_query_with_args! {
    #[doc = "Get the filtered vote delegatees."]
    get_filtered_vote_delegatees,
    GetFilteredVoteDelegatees,
    (val : StakeAddrs),
    BTreeMap<StakeAddr, DRep>,
}

block_query_with_args! {
    #[doc = "Query the SPO voting stake distribution"]
    get_spo_stake_distr,
    GetSPOStakeDistr,
    (val : Pools),
    BTreeMap<Addr, Coin>,
}

block_query_with_args! {
    #[doc = "Get proposals."]
    get_proposals,
    GetProposals,
    (val : TaggedSet<GovActionId>),
    Vec<GovActionState>,
}

block_query_with_args! {
    #[doc = "Get the state of committee members."]
    get_committee_members_state,
    GetCommitteeMembersState,
    (val1 : TaggedSet<Credential>, val2 : TaggedSet<Credential>, val3 : TaggedSet<MemberStatus>),
    CommitteeMembersState,
}

/// Macro to generate an async function with specific parameters and logic.
macro_rules! block_query_no_args {
    (
        $(#[doc = $doc:expr])*
        $fn_name:ident,
        $variant:ident,
        $type2:ty,
    ) => {
        $(#[doc = $doc])*
        pub async fn $fn_name(
            client: &mut Client,
            era: u16,
        ) -> Result<$type2, ClientError> {
            let query = BlockQuery::$variant;
            let query = LedgerQuery::BlockQuery(era, query);
            let query = Request::LedgerQuery(query);
            let (result,) = client.query(query).await?;

            Ok(result)
        }
    };
}

block_query_no_args! {
    #[doc = "Get the current protocol parameters"]
    get_current_pparams,
    GetCurrentPParams,
    CurrentProtocolParam,
}

block_query_no_args! {
    #[doc = "Get the block number for the current tip."]
    get_block_epoch_number,
    GetEpochNo,
    u32,
}

block_query_no_args! {
    #[doc = "Get the current stake distribution for the given era."]
    get_stake_distribution,
    GetStakeDistribution,
    StakeDistribution,
}

block_query_no_args! {
    #[doc = "Get the genesis configuration for the given era."]
    get_genesis_config,
    GetGenesisConfig,
    GenesisConfig,
}

block_query_no_args! {
    #[doc = "Get the /entire/ UTxO."]
    get_utxo_whole,
    GetUTxOWhole,
    UTxOWhole,
}

block_query_no_args! {
    #[doc = "Get the current Constitution."]
    get_constitution,
    GetConstitution,
    Constitution,
}

block_query_no_args! {
    #[doc = "Get the current governance state."]
    get_gov_state,
    GetGovState,
    GovState,
}

block_query_no_args! {
    #[doc = "Get the current account state."]
    get_account_state,
    GetAccountState,
    AccountState,
}

block_query_no_args! {
    #[doc = "Get the future protocol parameters. *Note*: It does **not** return [FuturePParams]."]
    get_future_protocol_params,
    GetFuturePParams,
    SMaybe<ProtocolParam>,
}

block_query_no_args! {
    #[doc = "Get the ratify state."]
    get_ratify_state,
    GetRatifyState,
    RatifyState,
}

block_query_no_args! {
    #[doc = "Get a snapshot of big ledger peers."]
    #[doc = ""]
    #[doc = "*Note*: This query (introduced by commit [ce08a04](https://github.com/IntersectMBO/ouroboros-consensus/commit/ce08a043e2bb6d6684375add5d347a9e023c1f1f) at [Ouroboros Consensus](https://github.com/IntersectMBO/ouroboros-consensus/blob/ce08a04/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Ledger/Query.hs#L325) has not been included in any node release yet."]
    get_big_ledger_snapshot,
    GetBigLedgerPeerSnapshot,
    LedgerPeerSnapshot,
}

block_query_no_args! {
    #[doc = "Get propoped updates to the protocol params."]
    get_proposed_pparams_updates,
    GetProposedPParamsUpdates,
    ProposedPPUpdates,
}
