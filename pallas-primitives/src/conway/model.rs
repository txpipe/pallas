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

use pallas_codec::minicbor::data::Type;

use std::collections::HashSet;

#[derive(Serialize, Deserialize, Encode, Debug, PartialEq, Eq, Clone)]
pub struct Multiasset<A>(#[n(0)] BTreeMap<PolicyId, BTreeMap<AssetName, A>>);

impl<'b, C, A: minicbor::Decode<'b, C>> minicbor::Decode<'b, C> for Multiasset<A> {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let policies: BTreeMap<PolicyId, BTreeMap<AssetName, A>> = d.decode_with(ctx)?;

        // In Conway, all policies must be nonempty, and all amounts must be nonzero.
        // We always parameterize Multiasset with NonZeroInt in practice, but maybe it should be
        // monomorphic?
        for (_policy, assets) in &policies {
            if assets.len() == 0 {
                return Err(minicbor::decode::Error::message("Policy must not be empty"));
            }
        }

        let result = Multiasset(policies);
        if !is_multiasset_small_enough(&result) {
            return Err(minicbor::decode::Error::message("Multiasset must not exceed size limit"));
        }
        Ok(result)
    }
}

pub type Mint = Multiasset<NonZeroInt>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Value {
    Coin(Coin),
    Multiasset(Coin, Multiasset<PositiveCoin>),
}

//codec_by_datatype! {
//    Value,
//    U8 | U16 | U32 | U64 => Coin,
//    (coin, multi => Multiasset)
//}

impl<C> minicbor::Encode<C> for Value {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Value::Coin(coin) => {
                e.encode(coin)?;
            },
            Value::Multiasset(coin, ma) => {
                e.array(2)?;
                e.encode(coin)?;
                e.encode(ma)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Value {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            Type::U8 | Type::U16 | Type::U32 | Type::U64 => {
                let coin = d.decode()?;
                Ok(Value::Coin(coin))
            }
            Type::Array | Type::ArrayIndef => {
                let _ = d.array()?;
                let coin = d.decode()?;
                let multiasset = d.decode()?;
                Ok(Value::Multiasset(coin, multiasset))
            }
            t => {
                Err(minicbor::decode::Error::message(format!("Unexpected datatype {}", t)))
            }
        }
    }
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

#[derive(Serialize, Deserialize, Encode, Debug, PartialEq, Clone)]
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
    pub auxiliary_data_hash: Option<Bytes>,

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

// This is ugly but I'm not sure how to do it with minicbor-derive
// the cbor map implementation is here:
// https://github.com/twittner/minicbor/blob/83a4a0f868ac9ffc924a282f8f917aa2ad7c698a/minicbor-derive/src/decode.rs#L405-L424
// We need to do validation inside the decoder or change the types of the validated fields to
// new types that do their own validation
impl<'b, C> minicbor::Decode<'b, C> for TransactionBody<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let mut must_inputs = None;
        let mut must_outputs = None;
        let mut must_fee = None;
        let mut ttl = None;
        let mut certificates = None;
        let mut withdrawals = None;
        let mut auxiliary_data_hash = None;
        let mut validity_interval_start = None;
        let mut mint: Option<Multiasset<NonZeroInt>> = None;
        let mut script_data_hash = None;
        let mut collateral = None;
        let mut required_signers = None;
        let mut network_id = None;
        let mut collateral_return = None;
        let mut total_collateral = None;
        let mut reference_inputs = None;
        let mut voting_procedures = None;
        let mut proposal_procedures = None;
        let mut treasury_value = None;
        let mut donation = None;

        let map_init = d.map()?;
        let mut items_seen = 0;

        let mut seen_key = HashSet::new();

        loop {
            let n = d.i64();
            let Ok(index) = n else { break };
            if seen_key.contains(&index) {
                return Err(minicbor::decode::Error::message("transaction body must not contain duplicate keys"));
            }
            match index {
                0 => {
                    must_inputs = d.decode_with(ctx)?;
                },
                1 => {
                    must_outputs = d.decode_with(ctx)?;
                },
                2 => {
                    must_fee = d.decode_with(ctx)?;
                },
                3 => {
                    ttl = d.decode_with(ctx)?;
                },
                4 => {
                    certificates = d.decode_with(ctx)?;
                },
                5 => {
                    let real_withdrawals: BTreeMap<RewardAccount, Coin> = d.decode_with(ctx)?;
                    if real_withdrawals.len() == 0 {
                        return Err(minicbor::decode::Error::message("withdrawals must be non-empty if present"));
                    }
                    withdrawals = Some(real_withdrawals);
                },
                7 => {
                    auxiliary_data_hash = d.decode_with(ctx)?;
                },
                8 => {
                    validity_interval_start = d.decode_with(ctx)?;
                },
                9 => {
                    let real_mint: Multiasset<NonZeroInt> = d.decode_with(ctx)?;
                    if real_mint.0.len() == 0 {
                        return Err(minicbor::decode::Error::message("mint must be non-empty if present"));
                    }
                    mint = Some(real_mint);
                },
                11 => {
                    script_data_hash = d.decode_with(ctx)?;
                },
                13 => {
                    collateral = d.decode_with(ctx)?;
                },
                14 => {
                    required_signers = d.decode_with(ctx)?;
                },
                15 => {
                    network_id = d.decode_with(ctx)?;
                },
                16 => {
                    collateral_return = d.decode_with(ctx)?;
                },
                17 => {
                    total_collateral = d.decode_with(ctx)?;
                },
                18 => {
                    reference_inputs = d.decode_with(ctx)?;
                },
                19 => {
                    let real_voting_procedures: VotingProcedures = d.decode_with(ctx)?;
                    if real_voting_procedures.len() == 0 {
                        return Err(minicbor::decode::Error::message("voting procedures must be non-empty if present"));
                    }
                    voting_procedures = Some(real_voting_procedures);
                },
                20 => {
                    let real_proposal_procedures: NonEmptySet<ProposalProcedure> = d.decode_with(ctx)?;
                    if real_proposal_procedures.len() == 0 {
                        return Err(minicbor::decode::Error::message("proposal procedures must be non-empty if present"));
                    }
                    proposal_procedures = Some(real_proposal_procedures);
                },
                21 => {
                    treasury_value = d.decode_with(ctx)?;
                },
                22 => {
                    donation = d.decode_with(ctx)?;
                },
                _ => {
                    return Err(minicbor::decode::Error::message("unexpected index"));
                }
            }
            seen_key.insert(index);
            items_seen += 1;
            if let Some(map_count) = map_init {
                if items_seen == map_count {
                    break;
                }
            }
        }

        if let Some(map_count) = map_init {
            if map_count != items_seen {
                return Err(minicbor::decode::Error::message("map is not valid cbor: declared count did not match actual count"));
            }
        } else {
            let ty = d.datatype()?;
            if ty == minicbor::data::Type::Break {
                d.skip()?;
            } else {
                return Err(minicbor::decode::Error::message("unexpected garbage at end of map"));
            }
        }

        let Some(inputs) = must_inputs else {
            return Err(minicbor::decode::Error::message("field inputs is required"));
        };
        let Some(outputs) = must_outputs else {
            return Err(minicbor::decode::Error::message("field outputs is required"));
        };
        let Some(fee) = must_fee else {
            return Err(minicbor::decode::Error::message("field fee is required"));
        };

        Ok(Self {
            inputs,
            outputs,
            fee,
            ttl,
            certificates,
            withdrawals,
            auxiliary_data_hash,
            validity_interval_start,
            mint,
            script_data_hash,
            collateral,
            required_signers,
            network_id,
            collateral_return,
            total_collateral,
            reference_inputs,
            voting_procedures,
            proposal_procedures,
            treasury_value,
            donation,
        })
    }
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
    pub plutus_data: Option<NonEmptySet<KeepRaw<'b, PlutusData>>>,

    #[n(5)]
    pub redeemer: Option<KeepRaw<'b, Redeemers>>,

    #[n(6)]
    pub plutus_v2_script: Option<NonEmptySet<PlutusScript<2>>>,

    #[n(7)]
    pub plutus_v3_script: Option<NonEmptySet<PlutusScript<3>>>,
}

//impl<'b, C> minicbor::Decode<'b, C> for WitnessSet<'b> {
//    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
//        let vkeywitness = d.decode_with(ctx)?;
//        let native_script = d.decode_with(ctx)?;
//        let bootstrap_witness = d.decode_with(ctx)?;
//        let plutus_v1_script = d.decode_with(ctx)?;
//        let plutus_data = d.decode_with(ctx)?;
//        let redeemer = d.decode_with(ctx)?;
//        let plutus_v2_script = d.decode_with(ctx)?;
//        let plutus_v3_script = d.decode_with(ctx)?;
//    }


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

// FIXME: re-exporting here means it does not use the above PostAlonzoAuxiliaryData; instead, it
// uses the one defined in the alonzo module, which only supports plutus V1 scripts
//
// Same problem exists in the babbage module
//
// should probably take a type parameter for the post-alonzo variant or just define a whole
// separate type here and in babbage
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

fn is_multiasset_small_enough<T>(ma: &Multiasset<T>) -> bool {
    let per_asset_size = 44;
    let per_policy_size = 28;

    let policy_count = ma.0.len();
    let mut asset_count = 0;
    for (_policy, assets) in &ma.0 {
        asset_count += assets.len();
    }

    let size = per_asset_size * asset_count + per_policy_size * policy_count;
    size <= 65535
}

#[cfg(test)]
mod tests {
    use super::Block;
    use pallas_codec::minicbor;

    type BlockWrapper<'b> = (u16, Block<'b>);

    #[cfg(test)]
    mod tests_value {
        use super::super::Mint;
        use super::super::Multiasset;
        use super::super::NonZeroInt;
        use super::super::Value;
        use pallas_codec::minicbor;
        use std::collections::BTreeMap;

        // a value can have zero coins and omit the multiasset
        #[test]
        fn decode_zero_value() {
            let ma: Value = minicbor::decode(&hex::decode("00").unwrap()).unwrap();
            assert_eq!(ma, Value::Coin(0));
        }

        // a value can have zero coins and an empty multiasset map
        // Note: this will roundtrip back to "00"
        #[test]
        fn permit_definite_value() {
            let ma: Value = minicbor::decode(&hex::decode("8200a0").unwrap()).unwrap();
            assert_eq!(ma, Value::Multiasset(0, Multiasset(BTreeMap::new())));
        }

        // Indefinite-encoded value is valid
        #[test]
        fn permit_indefinite_value() {
            let ma: Value = minicbor::decode(&hex::decode("9f00a0ff").unwrap()).unwrap();
            assert_eq!(ma, Value::Multiasset(0, Multiasset(BTreeMap::new())));
        }

        // the asset sub-map of a policy map in a multiasset must not be null in Conway
        #[test]
        fn reject_null_tokens() {
            let ma: Result<Value, _> = minicbor::decode(&hex::decode("8200a1581c00000000000000000000000000000000000000000000000000000000a0").unwrap());
            assert_eq!(
                ma.map_err(|e| e.to_string()),
                Err("decode error: Policy must not be empty".to_owned())
            );
        }

        // the asset sub-map of a policy map in a multiasset must not have any zero values in
        // Conway
        #[test]
        fn reject_zero_tokens() {
            let ma: Result<Value, _> = minicbor::decode(&hex::decode("8200a1581c00000000000000000000000000000000000000000000000000000000a14000").unwrap());
            assert_eq!(
                ma.map_err(|e| e.to_string()),
                Err("decode error: PositiveCoin must not be 0".to_owned())
            );
        }

        #[test]
        fn multiasset_reject_null_tokens() {
            let ma: Result<Multiasset<NonZeroInt>, _> = minicbor::decode(&hex::decode("a1581c00000000000000000000000000000000000000000000000000000000a0").unwrap());
            assert_eq!(
                ma.map_err(|e| e.to_string()),
                Err("decode error: Policy must not be empty".to_owned())
            );
        }

        // the decoder for MaryValue in the haskell node rejects inputs that are "too big" as
        // defined by `isMultiAssetSmallEnough`
        #[test]
        fn multiasset_not_too_big() {
            // Creating CBOR representation of a value with 1500 policies
            // 1500 * 44 is greater than 65535 so this should fail to decode
            let mut s: String = "b905dc".to_owned();
            for i in 0..1500u16 {
                // policy
                s += "581c0000000000000000000000000000000000000000000000000000";
                s += &hex::encode(i.to_be_bytes());
                // minimal token map (conway requires nonempty asset maps)
                s += "a14001";
            }
            let ma: Result<Multiasset<NonZeroInt>, _> = minicbor::decode(&hex::decode(s).unwrap());
            match ma {
                Ok(_) => panic!("decode succeded but should fail"),
                Err(e) => assert_eq!(e.to_string(), "decode error: Multiasset must not exceed size limit")
            }
        }

        #[test]
        fn mint_reject_null_tokens() {
            let ma: Result<Mint, _> = minicbor::decode(&hex::decode("a1581c00000000000000000000000000000000000000000000000000000000a0").unwrap());
            assert_eq!(
                ma.map_err(|e| e.to_string()),
                Err("decode error: Policy must not be empty".to_owned())
            );
        }
    }

    mod tests_witness_set {
        use super::super::{Bytes, VKeyWitness, WitnessSet};
        use pallas_codec::minicbor;

        #[test]
        fn decode_empty_witness_set() {
            let witness_set_bytes = hex::decode("a0").unwrap();
            let ws: WitnessSet = minicbor::decode(&witness_set_bytes).unwrap();
            assert_eq!(ws.vkeywitness, None);
        }

        // Legacy format is not supported when decoder version is 9
        // These tests should go in a pre-conway module?
        //#[test]
        //fn decode_witness_set_having_vkeywitness_legacy_may_be_empty() {
        //    let witness_set_bytes = hex::decode("a10080").unwrap();
        //    let ws: WitnessSet = minicbor::decode(&witness_set_bytes).unwrap();

        //    // FIXME: The decoder behavior here is strictly correct w.r.t. the haskell code; we
        //    // must accept a vkeywitness set that is present but empty (in the legacy witness set
        //    // format).
        //    //
        //    // However, the types we are using in pallas here are confusing; vkeywitness is of type
        //    // Option<NonEmptySet>, and in fact, our "NonEmptySet" type allows constructing an
        //    // empty value via CBOR decoding (there used to be a guard, but it was commented out).
        //    // So we end up with a Some(vec![]). It would make more sense to just have a 'Set'
        //    // type.
        //    assert_eq!(ws.vkeywitness.map(|s| s.to_vec()), Some(vec![]));
        //}

        //#[test]
        //fn decode_witness_set_having_vkeywitness_legacy_may_be_indefinite() {
        //    let witness_set_bytes = hex::decode("a1009fff").unwrap();
        //    let ws: WitnessSet = minicbor::decode(&witness_set_bytes).unwrap();

        //    assert_eq!(ws.vkeywitness.map(|s| s.to_vec()), Some(vec![]));
        //}

        //#[test]
        //fn decode_witness_set_having_vkeywitness_legacy_singleton() {
        //    let witness_set_bytes = hex::decode("a10081824040").unwrap();
        //    let ws: WitnessSet = minicbor::decode(&witness_set_bytes).unwrap();

        //    let expected = VKeyWitness {
        //        vkey: Bytes::from(vec![]),
        //        signature: Bytes::from(vec![]),
        //    };
        //    assert_eq!(ws.vkeywitness.map(|s| s.to_vec()), Some(vec![expected]));
        //}

        #[test]
        fn decode_witness_set_having_vkeywitness_untagged_must_be_nonempty() {
            let witness_set_bytes = hex::decode("a10080").unwrap();
            let ws: Result<WitnessSet, _> = minicbor::decode(&witness_set_bytes);
            assert_eq!(
                ws.map_err(|e| e.to_string()),
                Err("decode error: decoding empty set as NonEmptySet".to_owned())
            );
        }

        #[test]
        fn decode_witness_set_having_vkeywitness_untagged_singleton() {
            let witness_set_bytes = hex::decode("a10081824040").unwrap();
            let ws: WitnessSet = minicbor::decode(&witness_set_bytes).unwrap();

            let expected = VKeyWitness {
                vkey: Bytes::from(vec![]),
                signature: Bytes::from(vec![]),
            };
            assert_eq!(ws.vkeywitness.map(|s| s.to_vec()), Some(vec![expected]));
        }

        #[test]
        fn decode_witness_set_having_vkeywitness_conwaystyle_singleton() {
            let witness_set_bytes = hex::decode("a100d9010281824040").unwrap();
            let ws: WitnessSet = minicbor::decode(&witness_set_bytes).unwrap();

            let expected = VKeyWitness {
                vkey: Bytes::from(vec![]),
                signature: Bytes::from(vec![]),
            };
            assert_eq!(ws.vkeywitness.map(|s| s.to_vec()), Some(vec![expected]));
        }

        #[test]
        fn decode_witness_set_having_vkeywitness_conwaystyle_must_be_nonempty() {
            let witness_set_bytes = hex::decode("a100d9010280").unwrap();
            let ws: Result<WitnessSet, _> = minicbor::decode(&witness_set_bytes);
            assert_eq!(
                ws.map_err(|e| e.to_string()),
                Err("decode error: decoding empty set as NonEmptySet".to_owned())
            );
        }

        #[test]
        fn decode_witness_set_having_vkeywitness_reject_nonsense_tag() {
            // VKey witness set with nonsense tag 259
            let witness_set_bytes = hex::decode("a100d9010381824040").unwrap();
            let ws: Result<WitnessSet, _> = minicbor::decode(&witness_set_bytes);
            assert_eq!(
                ws.map_err(|e| e.to_string()),
                Err("decode error: Unrecognised tag: Tag(259)".to_owned())
            );
        }

        // Unclear what the behavior should be when there are duplicates. The haskell code
        // allows duplicate entries in the CBOR but represents the vkey witnesses using a
        // set data type, so that the resulting data structure will only have one element.
        // However, our NonEmptySet type is secretly a vector and does not prevent duplicates.
        // Do we ever hash witness sets? i.e. do we need to remember the original bytes?
        #[test]
        fn decode_witness_set_having_vkeywitness_duplicate_entries() {
            let witness_set_bytes = hex::decode("a100d9010282824040824040").unwrap();
            let ws: WitnessSet = minicbor::decode(&witness_set_bytes).unwrap();

            let expected = VKeyWitness {
                vkey: Bytes::from(vec![]),
                signature: Bytes::from(vec![]),
            };
            assert_eq!(ws.vkeywitness.map(|s| s.to_vec()), Some(vec![expected.clone(), expected]));
        }

    }

    mod tests_auxdata {
        use super::super::AuxiliaryData;
        use pallas_codec::minicbor;
        use std::collections::BTreeMap;

        #[test]
        fn decode_auxdata_shelley_format_empty() {
            let auxdata_bytes = hex::decode("a0").unwrap();
            let auxdata: AuxiliaryData =
                minicbor::decode(&auxdata_bytes).unwrap();
            match auxdata {
                AuxiliaryData::Shelley(s) => {
                    assert_eq!(s, BTreeMap::new());
                }
                _ => {
                    panic!("Unexpected variant");
                }
            }
        }

        #[test]
        fn decode_auxdata_shelley_ma_format_empty() {
            let auxdata_bytes = hex::decode("82a080").unwrap();
            let auxdata: AuxiliaryData =
                minicbor::decode(&auxdata_bytes).unwrap();
            match auxdata {
                AuxiliaryData::ShelleyMa(s) => {
                    assert_eq!(s.transaction_metadata, BTreeMap::new());
                }
                _ => {
                    panic!("Unexpected variant");
                }
            }
        }

        #[test]
        fn decode_auxdata_alonzo_format_empty() {
            let auxdata_bytes = hex::decode("d90103a0").unwrap();
            let auxdata: AuxiliaryData =
                minicbor::decode(&auxdata_bytes).unwrap();
            match auxdata {
                AuxiliaryData::PostAlonzo(a) => {
                    assert_eq!(a.metadata, None);
                }
                _ => {
                    panic!("Unexpected variant");
                }
            }
        }
    }

    mod tests_transaction {
        use super::super::TransactionBody;
        use pallas_codec::minicbor;

        // A simple tx with just inputs, outputs, and fee. Address is not well-formed, since the
        // 00 header implies both a payment part and a staking part are present.
        #[test]
        fn decode_simple_tx() {
            let tx_bytes = hex::decode("a300828258206767676767676767676767676767676767676767676767676767676767676767008258206767676767676767676767676767676767676767676767676767676767676767000200018182581c000000000000000000000000000000000000000000000000000000001a04000000").unwrap();
            let tx: TransactionBody = minicbor::decode(&tx_bytes).unwrap();
            assert_eq!(tx.fee, 0);
        }

        // The decoder for ConwayTxBodyRaw rejects transaction bodies missing inputs, outputs, or
        // fee
        #[test]
        fn reject_empty_tx() {
            let tx_bytes = hex::decode("a0").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: field inputs is required".to_owned())
            );
        }

        // Single input, no outputs, fee present but zero
        #[test]
        fn reject_tx_missing_outputs() {
            let tx_bytes = hex::decode("a200818258200000000000000000000000000000000000000000000000000000000000000008090200").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: field outputs is required".to_owned())
            );
        }

        // Single input, single output, no fee
        #[test]
        fn reject_tx_missing_fee() {
            let tx_bytes = hex::decode("a20081825820000000000000000000000000000000000000000000000000000000000000000809018182581c000000000000000000000000000000000000000000000000000000001affffffff").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: field fee is required".to_owned())
            );
        }

        // The mint may not be present if it is empty
        // TODO: equivalent tests for certs, withdrawals, collateral inputs, required signer
        // hashes, reference inputs, voting procedures, and proposal procedures
        #[test]
        fn reject_empty_present_mint() {
            let tx_bytes = hex::decode("a400828258206767676767676767676767676767676767676767676767676767676767676767008258206767676767676767676767676767676767676767676767676767676767676767000200018182581c000000000000000000000000000000000000000000000000000000001a0400000009a0").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: mint must be non-empty if present".to_owned())
            );
        }

        #[test]
        fn reject_empty_present_certs() {
            let tx_bytes = hex::decode("a400828258206767676767676767676767676767676767676767676767676767676767676767008258206767676767676767676767676767676767676767676767676767676767676767000200018182581c000000000000000000000000000000000000000000000000000000001a040000000480").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: decoding empty set as NonEmptySet".to_owned())
            );
        }

        #[test]
        fn reject_empty_present_withdrawals() {
            let tx_bytes = hex::decode("a400828258206767676767676767676767676767676767676767676767676767676767676767008258206767676767676767676767676767676767676767676767676767676767676767000200018182581c000000000000000000000000000000000000000000000000000000001a0400000005a0").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: withdrawals must be non-empty if present".to_owned())
            );
        }

        #[test]
        fn reject_empty_present_collateral_inputs() {
            let tx_bytes = hex::decode("a400828258206767676767676767676767676767676767676767676767676767676767676767008258206767676767676767676767676767676767676767676767676767676767676767000200018182581c000000000000000000000000000000000000000000000000000000001a040000000d80").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: decoding empty set as NonEmptySet".to_owned())
            );
        }

        #[test]
        fn reject_empty_present_required_signers() {
            let tx_bytes = hex::decode("a400828258206767676767676767676767676767676767676767676767676767676767676767008258206767676767676767676767676767676767676767676767676767676767676767000200018182581c000000000000000000000000000000000000000000000000000000001a040000000e80").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: decoding empty set as NonEmptySet".to_owned())
            );
        }

        #[test]
        fn reject_empty_present_voting_procedures() {
            let tx_bytes = hex::decode("a400828258206767676767676767676767676767676767676767676767676767676767676767008258206767676767676767676767676767676767676767676767676767676767676767000200018182581c000000000000000000000000000000000000000000000000000000001a0400000013a0").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: voting procedures must be non-empty if present".to_owned())
            );
        }

        #[test]
        fn reject_empty_present_proposal_procedures() {
            let tx_bytes = hex::decode("a400828258206767676767676767676767676767676767676767676767676767676767676767008258206767676767676767676767676767676767676767676767676767676767676767000200018182581c000000000000000000000000000000000000000000000000000000001a040000001480").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: decoding empty set as NonEmptySet".to_owned())
            );
        }

        #[test]
        fn reject_empty_present_donation() {
            let tx_bytes = hex::decode("a400828258206767676767676767676767676767676767676767676767676767676767676767008258206767676767676767676767676767676767676767676767676767676767676767000200018182581c000000000000000000000000000000000000000000000000000000001a040000001600").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: PositiveCoin must not be 0".to_owned())
            );
        }


        #[test]
        fn reject_duplicate_keys() {
            let tx_bytes = hex::decode("a40081825820000000000000000000000000000000000000000000000000000000000000000809018182581c000000000000000000000000000000000000000000000000000000001affffffff02010201").unwrap();
            let tx: Result<TransactionBody<'_>, _> = minicbor::decode(&tx_bytes);
            assert_eq!(
                tx.map_err(|e| e.to_string()),
                Err("decode error: transaction body must not contain duplicate keys".to_owned())
            );
        }
    }

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
