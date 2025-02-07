// TODO: this should move to pallas::ledger crate at some point

use pallas_crypto::hash::Hash;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::Hash as StdHash;
// required for derive attrs to work
use pallas_codec::minicbor::{self};

use pallas_codec::utils::{AnyUInt, Bytes, KeyValuePairs, Nullable, TagWrap};
use pallas_codec::{
    minicbor::{Decode, Encode},
    utils::AnyCbor,
};

pub mod primitives;

pub use primitives::{PoolMetadata, Relay};

use crate::miniprotocols::Point;

use crate::miniprotocols::localtxsubmission::SMaybe;

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
    GetStakePoolParams(Pools),
    GetRewardInfoPools,
    GetPoolState(SMaybe<Pools>),
    GetStakeSnapshots(SMaybe<Pools>),
    GetPoolDistr(SMaybe<Pools>),
    GetStakeDelegDeposits(AnyCbor),
    GetConstitution,
    GetGovState,
    GetDRepState(TaggedSet<Credential>),
    GetDRepStakeDistr(TaggedSet<DRep>),
    GetCommitteeMembersState(TaggedSet<Credential>, TaggedSet<Credential>, MemberStatus),
    GetFilteredVoteDelegatees(StakeAddrs),
    GetAccountState,
    GetSPOStakeDistr(Pools),
    GetProposals(TaggedSet<GovActionId>),
    GetRatifyState,
    GetFuturePParams,
    GetBigLedgerPeerSnapshot,
}

pub type Credential = StakeAddr;

/// TODO: Propoped updates to the protocol params as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/shelley/impl/src/Cardano/Ledger/Shelley/PParams.hs#L510-L511).
pub type ProposedPPUpdates = BTreeMap<Bytes, AnyCbor>;

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

/// TODO: Committee member state as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-api/src/Cardano/Ledger/Api/State/Query/CommitteeMembersState.hs#L106-L113). Not to be confused with plural [CommitteeMembersState].
pub type CommitteeMemberState = AnyCbor;

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

pub type MemberStatus = TaggedSet<AnyCbor>;

/// Action index as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L154).
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
#[cbor(transparent)]
pub struct GovActionIx {
    #[n(0)]
    pub index: u16,
}

/// Transaction ID as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/TxIn.hs#L56
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
#[cbor(transparent)]
pub struct TxId {
    #[n(0)]
    pub id: Bytes,
}

/// Governance action id as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L167-L170).
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct GovActionId {
    #[n(0)]
    pub tx_id: TxId,
    #[n(1)]
    pub gov_action_ix: GovActionIx,
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
/// Pool voting thresholds as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/PParams.hs#L223-L229).
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
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
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
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
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
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
    #[n(22)]
    pub pool_voting_thresholds: Option<PoolVotingThresholds>,
    #[n(23)]
    pub drep_voting_thresholds: Option<AnyCbor>,
    #[n(24)]
    pub committee_min_size: Option<u16>,
    #[n(25)]
    pub committee_max_term_length: Option<Epoch>,
    #[n(26)]
    pub gov_action_lifetime: Option<Epoch>,
    #[n(27)]
    pub gov_action_deposit: Option<Coin>,
    #[n(28)]
    pub drep_deposit: Option<Coin>,
    #[n(29)]
    pub drep_activity: Option<Epoch>,
    #[n(30)]
    pub min_fee_ref_script_cost_per_byte: Option<RationalNumber>,
}

#[derive(Debug, Encode, Decode, PartialEq)]
pub struct StakeDistribution {
    #[n(0)]
    pub pools: KeyValuePairs<Bytes, Pool>,
}

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

/// Stake controlled by a single pool, corresponding to [`IndividualPoolStake`](https://github.com/IntersectMBO/ouroboros-consensus/blob/e924f61d1fe63d25e9ecd8499b705aff4d553209/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Ledger/Query/Types.hs#L32-L35)
/// in the Haskell sources (not to be confused with [the `cardano-ledger` notion with the same name](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/PoolDistr.hs#L53-L61)).
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct IndividualPoolStake {
    #[n(0)]
    individual_pool_stake: RationalNumber,
    #[n(1)]
    individual_pool_stake_vrf: Bytes,
}

/// Map from pool hashes to [IndividualPoolStake]s, corresponding to [`PoolDistr`](https://github.com/IntersectMBO/ouroboros-consensus/blob/e924f61d1fe63d25e9ecd8499b705aff4d553209/ouroboros-consensus-cardano/src/shelley/Ouroboros/Consensus/Shelley/Ledger/Query/Types.hs#L62-L64)
/// in the Haskell sources (not to be confused with [the `cardano-ledger` notion with the same name](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/PoolDistr.hs#L100-L106)).
pub type PoolDistr = BTreeMap<Bytes, IndividualPoolStake>;

/// Anchor as [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/libs/cardano-ledger-core/src/Cardano/Ledger/BaseTypes.hs#L867-L870).
#[derive(Debug, Encode, Decode, PartialEq, Eq, Clone)]
pub struct Anchor {
    #[n(0)]
    pub url: String,
    #[n(1)]
    pub data_hash: Bytes,
}

/// Constitution as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L884-L887).
#[derive(Debug, Encode, Decode, PartialEq, Clone)]
pub struct Constitution {
    #[n(0)]
    pub anchor: Anchor,
    #[n(1)]
    pub script: Option<Bytes>,
}

/// TODO: Governance action state as defined [in the Haskell sources](https://github.com/IntersectMBO/cardano-ledger/blob/d30a7ae828e802e98277c82e278e570955afc273/eras/conway/impl/src/Cardano/Ledger/Conway/Governance/Procedures.hs#L211-L219).
pub type GovActionState = AnyCbor;

pub type GovRelation = AnyCbor;

/// TODO: Ledger peer snapshot as defined [in the Haskell sources](https://github.com/IntersectMBO/ouroboros-network/blob/df3431f95ef9e47a8a26fd3376efd61ed0837747/ouroboros-network-api/src/Ouroboros/Network/PeerSelection/LedgerPeers/Type.hs#L51-L53).
pub type LedgerPeerSnapshot = AnyCbor;

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
    pub enacted: GovActionState,
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
    DefinitePParamsUpdate(ProtocolParam),
    PotentialPParamsUpdate(SMaybe<ProtocolParam>),
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

/// Set of pool hashes.
///
/// The use of `BTreeMap`s (as per `TaggedSet` definition) ensures that the hashes are
/// in order (otherwise, the node will reject some queries).
pub type Pools = TaggedSet<Bytes>;

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
    pools: SMaybe<Pools>,
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

/// Get parameters for the given pools.
pub async fn get_stake_pool_params(
    client: &mut Client,
    era: u16,
    pools: Pools,
) -> Result<BTreeMap<Bytes, PoolParams>, ClientError> {
    let query = BlockQuery::GetStakePoolParams(pools);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result: (_,) = client.query(query).await?;

    Ok(result.0)
}

/// Get the current state of the given pools, or of all of them in case of a `SMaybe::None`.
pub async fn get_pool_state(
    client: &mut Client,
    era: u16,
    opt_pools: SMaybe<Pools>,
) -> Result<PState, ClientError> {
    let query = BlockQuery::GetPoolState(opt_pools);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result: (_,) = client.query(query).await?;

    Ok(result.0)
}

/// Get the current state of the given pools, or of all of them in case of a `SMaybe::None`.
pub async fn get_pool_distr(
    client: &mut Client,
    era: u16,
    opt_pools: SMaybe<Pools>,
) -> Result<PoolDistr, ClientError> {
    let query = BlockQuery::GetPoolDistr(opt_pools);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result: (_,) = client.query(query).await?;

    Ok(result.0)
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
pub async fn get_utxo_whole(client: &mut Client, era: u16) -> Result<UTxOWhole, ClientError> {
    let query = BlockQuery::GetUTxOWhole;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let result = client.query(query).await?;

    Ok(result)
}

/// Get the current Constitution.
pub async fn get_constitution(client: &mut Client, era: u16) -> Result<Constitution, ClientError> {
    let query = BlockQuery::GetConstitution;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Get the current governance state.
pub async fn get_gov_state(client: &mut Client, era: u16) -> Result<GovState, ClientError> {
    let query = BlockQuery::GetGovState;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Get the current DRep state.
pub async fn get_drep_state(
    client: &mut Client,
    era: u16,
    value: TaggedSet<StakeAddr>,
) -> Result<BTreeMap<StakeAddr, DRepState>, ClientError> {
    let query = BlockQuery::GetDRepState(value);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Get the current DRep stake distribution.
pub async fn get_drep_stake_distr(
    client: &mut Client,
    era: u16,
    value: TaggedSet<DRep>,
) -> Result<BTreeMap<DRep, Coin>, ClientError> {
    let query = BlockQuery::GetDRepStakeDistr(value);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Get the state of committee members.
pub async fn get_committee_members_state(
    client: &mut Client,
    era: u16,
    hot_credentials: TaggedSet<Credential>,
    cold_credentials: TaggedSet<Credential>,
    member_status: MemberStatus,
) -> Result<CommitteeMembersState, ClientError> {
    let query =
        BlockQuery::GetCommitteeMembersState(hot_credentials, cold_credentials, member_status);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Get the filtered vote delegatees.
pub async fn get_filtered_vote_delegatees(
    client: &mut Client,
    era: u16,
    stake_addrs: StakeAddrs,
) -> Result<BTreeMap<StakeAddr, DRep>, ClientError> {
    let query = BlockQuery::GetFilteredVoteDelegatees(stake_addrs);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Get the current account state.
pub async fn get_account_state(client: &mut Client, era: u16) -> Result<AccountState, ClientError> {
    let query = BlockQuery::GetAccountState;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Get the future protocol parameters. *Note*: It does **not** return [FuturePParams].
pub async fn get_future_protocol_params(
    client: &mut Client,
    era: u16,
) -> Result<SMaybe<ProtocolParam>, ClientError> {
    let query = BlockQuery::GetFuturePParams;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Query the SPO voting stake distribution
pub async fn get_spo_stake_distr(
    client: &mut Client,
    era: u16,
    pools: Pools,
) -> Result<BTreeMap<Addr, Coin>, ClientError> {
    let query = BlockQuery::GetSPOStakeDistr(pools);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Get proposals
pub async fn get_proposals(
    client: &mut Client,
    era: u16,
    action_ids: TaggedSet<GovActionId>,
) -> Result<Vec<GovActionState>, ClientError> {
    let query = BlockQuery::GetProposals(action_ids);
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Get the ratify state.
pub async fn get_ratify_state(client: &mut Client, era: u16) -> Result<RatifyState, ClientError> {
    let query = BlockQuery::GetRatifyState;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Get a snapshot of big ledger peers.
pub async fn get_big_ledger_snapshot(
    client: &mut Client,
    era: u16,
) -> Result<LedgerPeerSnapshot, ClientError> {
    let query = BlockQuery::GetBigLedgerPeerSnapshot;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}

/// Get propoped updates to the protocol params.
pub async fn get_proposed_pparams_updates(
    client: &mut Client,
    era: u16,
) -> Result<ProposedPPUpdates, ClientError> {
    let query = BlockQuery::GetProposedPParamsUpdates;
    let query = LedgerQuery::BlockQuery(era, query);
    let query = Request::LedgerQuery(query);
    let (result,) = client.query(query).await?;

    Ok(result)
}
