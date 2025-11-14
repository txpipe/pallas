//! Ledger primitives and cbor codec for the Conway era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Conway CDDL](https://github.com/IntersectMBO/cardano-ledger/blob/master/eras/conway/impl/cddl-files/conway.cddl) file in IntersectMBO repo.

use serde::{Deserialize, Serialize};

use pallas_codec::minicbor::{self, Decode, Encode};

pub use pallas_codec::codec_by_datatype;

pub use crate::{
    plutus_data::*, AddrKeyhash, AssetName, Bytes, Coin, CostModel, DnsName, Epoch, ExUnits,
    GenesisDelegateHash, Genesishash, Hash, IPv4, IPv6, KeepRaw, Metadata, Metadatum,
    MetadatumLabel, NetworkId, NonEmptySet, NonZeroInt, Nonce, NonceVariant, Nullable,
    PlutusScript, PolicyId, PoolKeyhash, PoolMetadata, PoolMetadataHash, Port, PositiveCoin,
    PositiveInterval, ProtocolVersion, RationalNumber, Relay, RewardAccount, ScriptHash, Set,
    StakeCredential, TransactionIndex, TransactionInput, UnitInterval, VrfCert, VrfKeyhash,
};

use crate::BTreeMap;

use crate::babbage;

pub use crate::babbage::HeaderBody;

pub use crate::babbage::OperationalCert;

pub use crate::babbage::Header;

pub type Multiasset<A> = BTreeMap<PolicyId, BTreeMap<AssetName, A>>;

pub type Mint = Multiasset<NonZeroInt>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Value {
    Coin(Coin),
    Multiasset(Coin, Multiasset<PositiveCoin>),
}

codec_by_datatype! {
    Value,
    U8 | U16 | U32 | U64 => Coin,
    (coin, multi => Multiasset)
}

pub use crate::alonzo::TransactionOutput as LegacyTransactionOutput;

pub type Withdrawals = BTreeMap<RewardAccount, Coin>;

pub type RequiredSigners = NonEmptySet<AddrKeyhash>;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cbor(flat)]
pub enum Certificate {
    #[n(0)]
    StakeRegistration(#[n(0)] StakeCredential),
    #[n(1)]
    StakeDeregistration(#[n(0)] StakeCredential),
    #[n(2)]
    StakeDelegation(#[n(0)] StakeCredential, #[n(1)] PoolKeyhash),
    #[n(3)]
    PoolRegistration {
        #[n(0)]
        operator: PoolKeyhash,
        #[n(1)]
        vrf_keyhash: VrfKeyhash,
        #[n(2)]
        pledge: Coin,
        #[n(3)]
        cost: Coin,
        #[n(4)]
        margin: UnitInterval,
        #[n(5)]
        reward_account: RewardAccount,
        #[n(6)]
        pool_owners: Set<AddrKeyhash>,
        #[n(7)]
        relays: Vec<Relay>,
        #[n(8)]
        pool_metadata: Option<PoolMetadata>,
    },
    #[n(4)]
    PoolRetirement(#[n(0)] PoolKeyhash, #[n(1)] Epoch),

    #[n(7)]
    Reg(#[n(0)] StakeCredential, #[n(1)] Coin),
    #[n(8)]
    UnReg(#[n(0)] StakeCredential, #[n(1)] Coin),
    #[n(9)]
    VoteDeleg(#[n(0)] StakeCredential, #[n(1)] DRep),
    #[n(10)]
    StakeVoteDeleg(#[n(0)] StakeCredential, #[n(1)] PoolKeyhash, #[n(2)] DRep),
    #[n(11)]
    StakeRegDeleg(#[n(0)] StakeCredential, #[n(1)] PoolKeyhash, #[n(2)] Coin),
    #[n(12)]
    VoteRegDeleg(#[n(0)] StakeCredential, #[n(1)] DRep, #[n(2)] Coin),
    #[n(13)]
    StakeVoteRegDeleg(
        #[n(0)] StakeCredential,
        #[n(1)] PoolKeyhash,
        #[n(2)] DRep,
        #[n(3)] Coin,
    ),

    #[n(14)]
    AuthCommitteeHot(
        #[n(0)] CommitteeColdCredential,
        #[n(1)] CommitteeHotCredential,
    ),
    #[n(15)]
    ResignCommitteeCold(#[n(0)] CommitteeColdCredential, #[n(1)] Option<Anchor>),
    #[n(16)]
    RegDRepCert(#[n(0)] DRepCredential, #[n(1)] Coin, #[n(2)] Option<Anchor>),
    #[n(17)]
    UnRegDRepCert(#[n(0)] DRepCredential, #[n(1)] Coin),
    #[n(18)]
    UpdateDRepCert(#[n(0)] DRepCredential, #[n(1)] Option<Anchor>),
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[cbor(flat)]
pub enum DRep {
    #[n(0)]
    Key(#[n(0)] AddrKeyhash),
    #[n(1)]
    Script(#[n(0)] ScriptHash),
    #[n(2)]
    Abstain,
    #[n(3)]
    NoConfidence,
}

pub type DRepCredential = StakeCredential;

pub type CommitteeColdCredential = StakeCredential;

pub type CommitteeHotCredential = StakeCredential;

#[derive(
    Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord, Hash,
)]
#[cbor(index_only)]
pub enum Language {
    #[n(0)]
    PlutusV1,

    #[n(1)]
    PlutusV2,

    #[n(2)]
    PlutusV3,
}

#[deprecated(since = "0.31.0", note = "use `CostModels` instead")]
pub type CostMdls = CostModels;

#[derive(Serialize, Deserialize, Encode, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct CostModels {
    #[n(0)]
    pub plutus_v1: Option<CostModel>,

    #[n(1)]
    pub plutus_v2: Option<CostModel>,

    #[n(2)]
    pub plutus_v3: Option<CostModel>,

    #[cbor(skip)]
    pub unknown: BTreeMap<u64, CostModel>,
}

impl<'b, C> minicbor::Decode<'b, C> for CostModels {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let models: BTreeMap<u64, CostModel> = d.decode_with(ctx)?;

        let mut plutus_v1 = None;
        let mut plutus_v2 = None;
        let mut plutus_v3 = None;
        let mut unknown: Vec<(u64, CostModel)> = Vec::new();

        for (k, v) in models.iter() {
            match k {
                0 => plutus_v1 = Some(v.clone()),
                1 => plutus_v2 = Some(v.clone()),
                2 => plutus_v3 = Some(v.clone()),
                _ => unknown.push((*k, v.clone())),
            }
        }

        Ok(Self {
            plutus_v1,
            plutus_v2,
            plutus_v3,
            unknown: unknown.into_iter().collect(),
        })
    }
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct ProtocolParamUpdate {
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

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Update {
    #[n(0)]
    pub proposed_protocol_parameter_updates: BTreeMap<Genesishash, ProtocolParamUpdate>,

    #[n(1)]
    pub epoch: Epoch,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
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
    pub security_voting_threshold: UnitInterval,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct DRepVotingThresholds {
    #[n(0)]
    pub motion_no_confidence: UnitInterval,
    #[n(1)]
    pub committee_normal: UnitInterval,
    #[n(2)]
    pub committee_no_confidence: UnitInterval,
    #[n(3)]
    pub update_constitution: UnitInterval,
    #[n(4)]
    pub hard_fork_initiation: UnitInterval,
    #[n(5)]
    pub pp_network_group: UnitInterval,
    #[n(6)]
    pub pp_economic_group: UnitInterval,
    #[n(7)]
    pub pp_technical_group: UnitInterval,
    #[n(8)]
    pub pp_governance_group: UnitInterval,
    #[n(9)]
    pub treasury_withdrawal: UnitInterval,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct TransactionBody<'a> {
    #[n(0)]
    pub inputs: Set<TransactionInput>,

    #[b(1)]
    pub outputs: Vec<TransactionOutput<'a>>,

    #[n(2)]
    pub fee: Coin,

    #[n(3)]
    pub ttl: Option<u64>,

    #[n(4)]
    pub certificates: Option<NonEmptySet<Certificate>>,

    #[n(5)]
    pub withdrawals: Option<BTreeMap<RewardAccount, Coin>>,

    #[n(7)]
    pub auxiliary_data_hash: Option<Hash<32>>,

    #[n(8)]
    pub validity_interval_start: Option<u64>,

    #[n(9)]
    pub mint: Option<Multiasset<NonZeroInt>>,

    #[n(11)]
    pub script_data_hash: Option<Hash<32>>,

    #[n(13)]
    pub collateral: Option<NonEmptySet<TransactionInput>>,

    #[n(14)]
    pub required_signers: Option<RequiredSigners>,

    #[n(15)]
    pub network_id: Option<NetworkId>,

    #[n(16)]
    pub collateral_return: Option<TransactionOutput<'a>>,

    #[n(17)]
    pub total_collateral: Option<Coin>,

    #[n(18)]
    pub reference_inputs: Option<NonEmptySet<TransactionInput>>,

    // -- NEW IN CONWAY
    #[n(19)]
    pub voting_procedures: Option<VotingProcedures>,

    #[n(20)]
    pub proposal_procedures: Option<NonEmptySet<ProposalProcedure>>,

    #[n(21)]
    pub treasury_value: Option<Coin>,

    #[n(22)]
    pub donation: Option<PositiveCoin>,
}

#[deprecated(since = "1.0.0-alpha", note = "use `TransactionBody` instead")]
pub type MintedTransactionBody<'a> = TransactionBody<'a>;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cbor(index_only)]
pub enum Vote {
    #[n(0)]
    No,
    #[n(1)]
    Yes,
    #[n(2)]
    Abstain,
}

pub type VotingProcedures = BTreeMap<Voter, BTreeMap<GovActionId, VotingProcedure>>;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct VotingProcedure {
    #[n(0)]
    pub vote: Vote,
    #[n(1)]
    pub anchor: Option<Anchor>,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ProposalProcedure {
    #[n(0)]
    pub deposit: Coin,
    #[n(1)]
    pub reward_account: RewardAccount,
    #[n(2)]
    pub gov_action: GovAction,
    #[n(3)]
    pub anchor: Anchor,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cbor(flat)]
pub enum GovAction {
    #[n(0)]
    ParameterChange(
        #[n(0)] Option<GovActionId>,
        #[n(1)] Box<ProtocolParamUpdate>,
        #[n(2)] Option<ScriptHash>,
    ),
    #[n(1)]
    HardForkInitiation(#[n(0)] Option<GovActionId>, #[n(1)] ProtocolVersion),
    #[n(2)]
    TreasuryWithdrawals(
        #[n(0)] BTreeMap<RewardAccount, Coin>,
        #[n(1)] Option<ScriptHash>,
    ),
    #[n(3)]
    NoConfidence(#[n(0)] Option<GovActionId>),
    #[n(4)]
    UpdateCommittee(
        #[n(0)] Option<GovActionId>,
        #[n(1)] Set<CommitteeColdCredential>,
        #[n(2)] BTreeMap<CommitteeColdCredential, Epoch>,
        #[n(3)] UnitInterval,
    ),
    #[n(5)]
    NewConstitution(#[n(0)] Option<GovActionId>, #[n(1)] Constitution),
    #[n(6)]
    Information,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Constitution {
    #[n(0)]
    pub anchor: Anchor,
    #[n(1)]
    pub guardrail_script: Option<ScriptHash>,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[cbor(flat)]
pub enum Voter {
    #[n(1)]
    ConstitutionalCommitteeScript(#[n(0)] ScriptHash),
    #[n(0)]
    ConstitutionalCommitteeKey(#[n(0)] AddrKeyhash),
    #[n(3)]
    DRepScript(#[n(0)] ScriptHash),
    #[n(2)]
    DRepKey(#[n(0)] AddrKeyhash),
    #[n(4)]
    StakePoolKey(#[n(0)] AddrKeyhash),
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct Anchor {
    #[n(0)]
    pub url: String,
    #[n(1)]
    pub content_hash: Hash<32>,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct GovActionId {
    #[n(0)]
    pub transaction_id: Hash<32>,
    #[n(1)]
    pub action_index: u32,
}

pub type PostAlonzoTransactionOutput<'b> =
    babbage::GenPostAlonzoTransactionOutput<'b, Value, ScriptRef<'b>>;

#[deprecated(
    since = "1.0.0-alpha",
    note = "use `PostAlonzoTransactionOutput` instead"
)]
pub type MintedPostAlonzoTransactionOutput<'b> = PostAlonzoTransactionOutput<'b>;

pub type TransactionOutput<'b> = babbage::GenTransactionOutput<'b, PostAlonzoTransactionOutput<'b>>;

// FIXME: Repeated since macro does not handle type generics yet.
codec_by_datatype! {
    TransactionOutput<'b>,
    Array | ArrayIndef => Legacy,
    Map | MapIndef => PostAlonzo,
    ()
}

#[deprecated(since = "1.0.0-alpha", note = "use `TransactionOutput` instead")]
pub type MintedTransactionOutput<'b> = TransactionOutput<'b>;

pub use crate::alonzo::VKeyWitness;

pub use crate::alonzo::NativeScript;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct ExUnitPrices {
    #[n(0)]
    pub mem_price: RationalNumber,

    #[n(1)]
    pub step_price: RationalNumber,
}

#[derive(
    Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord,
)]
#[cbor(index_only)]
pub enum RedeemerTag {
    #[n(0)]
    Spend,
    #[n(1)]
    Mint,
    #[n(2)]
    Cert,
    #[n(3)]
    Reward,
    #[n(4)]
    Vote,
    #[n(5)]
    Propose,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Redeemer {
    #[n(0)]
    pub tag: RedeemerTag,

    #[n(1)]
    pub index: u32,

    #[n(2)]
    pub data: PlutusData,

    #[n(3)]
    pub ex_units: ExUnits,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct RedeemersKey {
    #[n(0)]
    pub tag: RedeemerTag,
    #[n(1)]
    pub index: u32,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct RedeemersValue {
    #[n(0)]
    pub data: PlutusData,
    #[n(1)]
    pub ex_units: ExUnits,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Redeemers {
    List(Vec<Redeemer>),
    Map(BTreeMap<RedeemersKey, RedeemersValue>),
}

codec_by_datatype! {
    Redeemers,
    Array | ArrayIndef => List,
    Map | MapIndef => Map,
    ()
}

impl From<BTreeMap<RedeemersKey, RedeemersValue>> for Redeemers {
    fn from(value: BTreeMap<RedeemersKey, RedeemersValue>) -> Self {
        Redeemers::Map(value)
    }
}

pub use crate::alonzo::BootstrapWitness;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct WitnessSet<'b> {
    #[n(0)]
    pub vkeywitness: Option<NonEmptySet<VKeyWitness>>,

    #[n(1)]
    pub native_script: Option<NonEmptySet<KeepRaw<'b, NativeScript>>>,

    #[n(2)]
    pub bootstrap_witness: Option<NonEmptySet<BootstrapWitness>>,

    #[n(3)]
    pub plutus_v1_script: Option<NonEmptySet<PlutusScript<1>>>,

    #[b(4)]
    pub plutus_data: Option<KeepRaw<'b, NonEmptySet<KeepRaw<'b, PlutusData>>>>,

    #[n(5)]
    pub redeemer: Option<KeepRaw<'b, Redeemers>>,

    #[n(6)]
    pub plutus_v2_script: Option<NonEmptySet<PlutusScript<2>>>,

    #[n(7)]
    pub plutus_v3_script: Option<NonEmptySet<PlutusScript<3>>>,
}

#[deprecated(since = "1.0.0-alpha", note = "use `WitnessSet` instead")]
pub type MintedWitnessSet<'b> = WitnessSet<'b>;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct PostAlonzoAuxiliaryData {
    #[n(0)]
    pub metadata: Option<Metadata>,

    #[n(1)]
    pub native_scripts: Option<Vec<NativeScript>>,

    #[n(2)]
    pub plutus_v1_scripts: Option<Vec<PlutusScript<1>>>,

    #[n(3)]
    pub plutus_v2_scripts: Option<Vec<PlutusScript<2>>>,

    #[n(4)]
    pub plutus_v3_scripts: Option<Vec<PlutusScript<3>>>,
}

pub use babbage::DatumHash;

pub use babbage::DatumOption;

#[deprecated(since = "0.31.0", note = "use `PlutusScript<1>` instead")]
pub type PlutusV1Script = PlutusScript<1>;

#[deprecated(since = "0.31.0", note = "use `PlutusScript<2>` instead")]
pub type PlutusV2Script = PlutusScript<2>;

#[deprecated(since = "0.31.0", note = "use `PlutusScript<3>` instead")]
pub type PlutusV3Script = PlutusScript<3>;

// script = [0, native_script // 1, plutus_v1_script // 2, plutus_v2_script //
// 3, plutus_v3_script]
#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cbor(flat)]
pub enum ScriptRef<'b> {
    #[n(0)]
    NativeScript(#[b(0)] KeepRaw<'b, NativeScript>),
    #[n(1)]
    PlutusV1Script(#[n(0)] PlutusScript<1>),
    #[n(2)]
    PlutusV2Script(#[n(0)] PlutusScript<2>),
    #[n(3)]
    PlutusV3Script(#[n(0)] PlutusScript<3>),
}

// TODO: Remove in favour of multierascriptref
impl<'b> From<babbage::ScriptRef<'b>> for ScriptRef<'b> {
    fn from(value: babbage::ScriptRef<'b>) -> Self {
        match value {
            babbage::ScriptRef::NativeScript(x) => Self::NativeScript(x),
            babbage::ScriptRef::PlutusV1Script(x) => Self::PlutusV1Script(x),
            babbage::ScriptRef::PlutusV2Script(x) => Self::PlutusV2Script(x),
        }
    }
}

#[deprecated(since = "1.0.0-alpha", note = "use `ScriptRef` instead")]
pub type MintedScriptRef<'b> = ScriptRef<'b>;

pub use crate::alonzo::AuxiliaryData;

/// A memory representation of an already minted block
///
/// This structure allows to retrieve the
/// original CBOR bytes for each structure that might require hashing. In this
/// way, we make sure that the resulting hash matches what exists on-chain.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
pub struct Block<'b> {
    #[n(0)]
    pub header: KeepRaw<'b, Header>,

    #[b(1)]
    pub transaction_bodies: Vec<KeepRaw<'b, TransactionBody<'b>>>,

    #[n(2)]
    pub transaction_witness_sets: Vec<KeepRaw<'b, WitnessSet<'b>>>,

    #[n(3)]
    pub auxiliary_data_set: BTreeMap<TransactionIndex, KeepRaw<'b, AuxiliaryData>>,

    #[n(4)]
    pub invalid_transactions: Option<Vec<TransactionIndex>>,
}

#[deprecated(since = "1.0.0-alpha", note = "use `Block` instead")]
pub type MintedBlock<'b> = Block<'b>;

#[derive(Clone, Serialize, Deserialize, Encode, Decode, Debug)]
pub struct Tx<'b> {
    #[b(0)]
    pub transaction_body: KeepRaw<'b, TransactionBody<'b>>,

    #[n(1)]
    pub transaction_witness_set: KeepRaw<'b, WitnessSet<'b>>,

    #[n(2)]
    pub success: bool,

    #[n(3)]
    pub auxiliary_data: Nullable<KeepRaw<'b, AuxiliaryData>>,
}

#[deprecated(since = "1.0.0-alpha", note = "use `Tx` instead")]
pub type MintedTx<'b> = Tx<'b>;

#[cfg(test)]
mod tests {
    use super::Block;
    use pallas_codec::minicbor;

    type BlockWrapper<'b> = (u16, Block<'b>);

    #[cfg(test)]
    mod tests_voter {
        use super::super::Voter;
        use crate::Hash;
        use std::cmp::Ordering;
        use test_case::test_case;

        fn fake_hash(prefix: &str) -> Hash<28> {
            let null_hash: [u8; 28] = [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ];
            Hash::from(&[prefix.as_bytes(), &null_hash].concat()[0..28])
        }

        fn cc_script(prefix: &str) -> Voter {
            Voter::ConstitutionalCommitteeScript(fake_hash(prefix))
        }

        fn cc_key(prefix: &str) -> Voter {
            Voter::ConstitutionalCommitteeKey(fake_hash(prefix))
        }

        fn drep_script(prefix: &str) -> Voter {
            Voter::DRepScript(fake_hash(prefix))
        }

        fn drep_key(prefix: &str) -> Voter {
            Voter::DRepKey(fake_hash(prefix))
        }

        fn spo(prefix: &str) -> Voter {
            Voter::StakePoolKey(fake_hash(prefix))
        }

        #[test_case(cc_script("alice"), cc_script("alice") => Ordering::Equal)]
        #[test_case(cc_script("alice"), cc_key("alice") => Ordering::Less)]
        #[test_case(cc_script("alice"), drep_script("alice") => Ordering::Less)]
        #[test_case(cc_script("alice"), drep_key("alice") => Ordering::Less)]
        #[test_case(cc_script("alice"), spo("alice") => Ordering::Less)]
        #[test_case(cc_script("bob"), cc_script("alice") => Ordering::Greater)]
        #[test_case(drep_script("alice"), cc_script("alice") => Ordering::Greater)]
        #[test_case(drep_script("alice"), cc_key("alice") => Ordering::Greater)]
        #[test_case(drep_script("alice"), drep_script("alice") => Ordering::Equal)]
        #[test_case(drep_script("alice"), drep_key("alice") => Ordering::Less)]
        #[test_case(drep_script("alice"), spo("alice") => Ordering::Less)]
        #[test_case(drep_script("bob"), drep_script("alice") => Ordering::Greater)]
        fn voter_ordering(left: Voter, right: Voter) -> Ordering {
            left.cmp(&right)
        }
    }

    #[test]
    fn block_isomorphic_decoding_encoding() {
        let test_blocks = [
            include_str!("../../../test_data/conway1.block"),
            include_str!("../../../test_data/conway2.block"),
            // interesting block with extreme values
            include_str!("../../../test_data/conway3.block"),
            // interesting block with extreme values
            include_str!("../../../test_data/conway4.block"),
        ];

        for (idx, block_str) in test_blocks.iter().enumerate() {
            println!("decoding test block {}", idx + 1);
            let bytes = hex::decode(block_str).unwrap_or_else(|_| panic!("bad block file {idx}"));

            let block: BlockWrapper = minicbor::decode(&bytes)
                .unwrap_or_else(|e| panic!("error decoding cbor for file {idx}: {e:?}"));

            let bytes2 = minicbor::to_vec(block)
                .unwrap_or_else(|e| panic!("error encoding block cbor for file {idx}: {e:?}"));

            assert!(bytes.eq(&bytes2), "re-encoded bytes didn't match original");
        }
    }

    // #[test]
    // fn fragments_decoding() {
    //     // peculiar array of outputs used in an hydra transaction
    //     let bytes = hex::decode(hex).unwrap();
    //     let outputs =
    // Vec::<TransactionOutput>::decode_fragment(&bytes).unwrap();
    //
    //     dbg!(outputs);
    //
    //     // add any loose fragment tests here
    // }
}
