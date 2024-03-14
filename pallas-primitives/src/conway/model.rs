//! Ledger primitives and cbor codec for the Conway era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Conway CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/conway/test-suite/cddl-files/conway.cddl) file in IOHK repo.

use std::ops::Deref;

use pallas_codec::minicbor::decode::Error;
use serde::{Deserialize, Serialize};

use pallas_codec::minicbor::{Decode, Encode};
use pallas_crypto::hash::Hash;

use pallas_codec::utils::{
    Bytes, KeepRaw, KeyValuePairs, MaybeIndefArray, NonEmptyKeyValuePairs, NonEmptySet, NonZeroInt,
    Nullable, PositiveCoin, Set,
};

// required for derive attrs to work
use pallas_codec::minicbor;

pub use crate::alonzo::VrfCert;

pub use crate::babbage::HeaderBody;

pub use crate::babbage::OperationalCert;

pub use crate::alonzo::ProtocolVersion;

pub use crate::alonzo::KesSignature;

pub use crate::babbage::Header;

pub use crate::alonzo::TransactionInput;

pub use crate::alonzo::NonceVariant;

pub use crate::alonzo::Nonce;

pub use crate::alonzo::ScriptHash;

pub use crate::alonzo::PolicyId;

pub use crate::alonzo::AssetName;

pub type Multiasset<A> = NonEmptyKeyValuePairs<PolicyId, KeyValuePairs<AssetName, A>>;

pub use crate::alonzo::Mint;

pub use crate::alonzo::Coin;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Value {
    Coin(Coin),
    Multiasset(Coin, Multiasset<PositiveCoin>),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for Value {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::U8 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U16 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U32 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::U64 => Ok(Value::Coin(d.decode_with(ctx)?)),
            minicbor::data::Type::Array => {
                d.array()?;
                let coin = d.decode_with(ctx)?;
                let multiasset = d.decode_with(ctx)?;
                Ok(Value::Multiasset(coin, multiasset))
            }
            _ => Err(minicbor::decode::Error::message(
                "unknown cbor data type for Alonzo Value enum",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for Value {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        // TODO: check how to deal with uint variants (u32 vs u64)
        match self {
            Value::Coin(coin) => {
                e.encode_with(coin, ctx)?;
            }
            Value::Multiasset(coin, other) => {
                e.array(2)?;
                e.encode_with(coin, ctx)?;
                e.encode_with(other, ctx)?;
            }
        };

        Ok(())
    }
}

pub use crate::alonzo::TransactionOutput as LegacyTransactionOutput;

pub use crate::alonzo::PoolKeyhash;

pub use crate::alonzo::Epoch;

pub use crate::alonzo::Genesishash;

pub use crate::alonzo::GenesisDelegateHash;

pub use crate::alonzo::VrfKeyhash;

pub use crate::alonzo::InstantaneousRewardSource;

pub use crate::alonzo::InstantaneousRewardTarget;

pub use crate::alonzo::MoveInstantaneousReward;

pub use crate::alonzo::RewardAccount;

pub type Withdrawals = NonEmptyKeyValuePairs<RewardAccount, Coin>;

pub type RequiredSigners = NonEmptySet<AddrKeyhash>;

pub use crate::alonzo::Port;

pub use crate::alonzo::IPv4;

pub use crate::alonzo::IPv6;

pub use crate::alonzo::DnsName;

pub use crate::alonzo::Relay;

pub use crate::alonzo::PoolMetadataHash;

pub use crate::alonzo::PoolMetadata;

pub use crate::alonzo::AddrKeyhash;

pub use crate::alonzo::Scripthash;

pub use crate::alonzo::RationalNumber;

pub use crate::alonzo::UnitInterval;

pub use crate::alonzo::PositiveInterval;

pub use crate::alonzo::StakeCredential;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Certificate {
    StakeRegistration(StakeCredential),
    StakeDeregistration(StakeCredential),
    StakeDelegation(StakeCredential, PoolKeyhash),
    PoolRegistration {
        operator: PoolKeyhash,
        vrf_keyhash: VrfKeyhash,
        pledge: Coin,
        cost: Coin,
        margin: UnitInterval,
        reward_account: RewardAccount,
        pool_owners: Set<AddrKeyhash>,
        relays: Vec<Relay>,
        pool_metadata: Option<PoolMetadata>,
    },
    PoolRetirement(PoolKeyhash, Epoch),

    Reg(StakeCredential, Coin),
    UnReg(StakeCredential, Coin),
    VoteDeleg(StakeCredential, DRep),
    StakeVoteDeleg(StakeCredential, PoolKeyhash, DRep),
    StakeRegDeleg(StakeCredential, PoolKeyhash, Coin),
    VoteRegDeleg(StakeCredential, DRep, Coin),
    StakeVoteRegDeleg(StakeCredential, PoolKeyhash, DRep, Coin),

    AuthCommitteeHot(CommitteeColdCredential, CommitteeHotCredential),
    ResignCommitteeCold(CommitteeColdCredential),
    RegDRepCert(DRepCredential, Coin, Option<Anchor>),
    UnRegDRepCert(DRepCredential, Coin),
    UpdateDRepCert(StakeCredential, Option<Anchor>),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for Certificate {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => {
                let a = d.decode_with(ctx)?;
                Ok(Certificate::StakeRegistration(a))
            }
            1 => {
                let a = d.decode_with(ctx)?;
                Ok(Certificate::StakeDeregistration(a))
            }
            2 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::StakeDelegation(a, b))
            }
            3 => {
                let operator = d.decode_with(ctx)?;
                let vrf_keyhash = d.decode_with(ctx)?;
                let pledge = d.decode_with(ctx)?;
                let cost = d.decode_with(ctx)?;
                let margin = d.decode_with(ctx)?;
                let reward_account = d.decode_with(ctx)?;
                let pool_owners = d.decode_with(ctx)?;
                let relays = d.decode_with(ctx)?;
                let pool_metadata = d.decode_with(ctx)?;

                Ok(Certificate::PoolRegistration {
                    operator,
                    vrf_keyhash,
                    pledge,
                    cost,
                    margin,
                    reward_account,
                    pool_owners,
                    relays,
                    pool_metadata,
                })
            }
            4 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::PoolRetirement(a, b))
            }

            7 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::Reg(a, b))
            }
            8 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::UnReg(a, b))
            }
            9 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::VoteDeleg(a, b))
            }
            10 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                Ok(Certificate::StakeVoteDeleg(a, b, c))
            }
            11 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                Ok(Certificate::StakeRegDeleg(a, b, c))
            }
            12 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                Ok(Certificate::VoteRegDeleg(a, b, c))
            }
            13 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                let d = d.decode_with(ctx)?;
                Ok(Certificate::StakeVoteRegDeleg(a, b, c, d))
            }
            14 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::AuthCommitteeHot(a, b))
            }
            15 => {
                let a = d.decode_with(ctx)?;
                Ok(Certificate::ResignCommitteeCold(a))
            }
            16 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                Ok(Certificate::RegDRepCert(a, b, c))
            }
            17 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::UnRegDRepCert(a, b))
            }
            18 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(Certificate::UpdateDRepCert(a, b))
            }
            _ => Err(minicbor::decode::Error::message(
                "unknown variant id for certificate",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for Certificate {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Certificate::StakeRegistration(a) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode_with(a, ctx)?;
            }
            Certificate::StakeDeregistration(a) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode_with(a, ctx)?;
            }
            Certificate::StakeDelegation(a, b) => {
                e.array(3)?;
                e.u16(2)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::PoolRegistration {
                operator,
                vrf_keyhash,
                pledge,
                cost,
                margin,
                reward_account,
                pool_owners,
                relays,
                pool_metadata,
            } => {
                e.array(10)?;
                e.u16(3)?;

                e.encode_with(operator, ctx)?;
                e.encode_with(vrf_keyhash, ctx)?;
                e.encode_with(pledge, ctx)?;
                e.encode_with(cost, ctx)?;
                e.encode_with(margin, ctx)?;
                e.encode_with(reward_account, ctx)?;
                e.encode_with(pool_owners, ctx)?;
                e.encode_with(relays, ctx)?;
                e.encode_with(pool_metadata, ctx)?;
            }
            Certificate::PoolRetirement(a, b) => {
                e.array(3)?;
                e.u16(4)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            // 5 and 6 removed in conway
            Certificate::Reg(a, b) => {
                e.array(3)?;
                e.u16(7)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::UnReg(a, b) => {
                e.array(3)?;
                e.u16(8)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::VoteDeleg(a, b) => {
                e.array(3)?;
                e.u16(9)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::StakeVoteDeleg(a, b, c) => {
                e.array(4)?;
                e.u16(10)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::StakeRegDeleg(a, b, c) => {
                e.array(4)?;
                e.u16(11)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::VoteRegDeleg(a, b, c) => {
                e.array(4)?;
                e.u16(12)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::StakeVoteRegDeleg(a, b, c, d) => {
                e.array(5)?;
                e.u16(13)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
                e.encode_with(d, ctx)?;
            }
            Certificate::AuthCommitteeHot(a, b) => {
                e.array(3)?;
                e.u16(14)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::ResignCommitteeCold(a) => {
                e.array(2)?;
                e.u16(15)?;
                e.encode_with(a, ctx)?;
            }
            Certificate::RegDRepCert(a, b, c) => {
                e.array(4)?;
                e.u16(16)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::UnRegDRepCert(a, b) => {
                e.array(3)?;
                e.u16(17)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::UpdateDRepCert(a, b) => {
                e.array(3)?;
                e.u16(18)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum DRep {
    Key(AddrKeyhash),
    Script(Scripthash),
    Abstain,
    NoConfidence,
}

impl<'b, C> minicbor::decode::Decode<'b, C> for DRep {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => Ok(DRep::Key(d.decode_with(ctx)?)),
            1 => Ok(DRep::Script(d.decode_with(ctx)?)),
            2 => Ok(DRep::Abstain),
            3 => Ok(DRep::NoConfidence),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant id for DRep",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for DRep {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            DRep::Key(h) => {
                e.array(2)?;
                e.encode_with(0, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            DRep::Script(h) => {
                e.array(2)?;
                e.encode_with(1, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            DRep::Abstain => {
                e.array(1)?;
                e.encode_with(2, ctx)?;

                Ok(())
            }
            DRep::NoConfidence => {
                e.array(1)?;
                e.encode_with(3, ctx)?;

                Ok(())
            }
        }
    }
}

pub type DRepCredential = StakeCredential;

pub type CommitteeColdCredential = StakeCredential;

pub type CommitteeHotCredential = StakeCredential;

pub use crate::alonzo::NetworkId;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(index_only)]
pub enum Language {
    #[n(0)]
    PlutusV1,

    #[n(1)]
    PlutusV2,

    #[n(2)]
    PlutusV3,
}

pub use crate::alonzo::CostModel;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct CostMdls {
    #[n(0)]
    pub plutus_v1: Option<CostModel>,

    #[n(1)]
    pub plutus_v2: Option<CostModel>,

    #[n(2)]
    pub plutus_v3: Option<CostModel>,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct ProtocolParamUpdate {
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

    #[n(16)]
    pub min_pool_cost: Option<Coin>,
    #[n(17)]
    pub ada_per_utxo_byte: Option<Coin>,
    #[n(18)]
    pub cost_models_for_script_languages: Option<CostMdls>,
    #[n(19)]
    pub execution_costs: Option<ExUnitPrices>,
    #[n(20)]
    pub max_tx_ex_units: Option<ExUnits>,
    #[n(21)]
    pub max_block_ex_units: Option<ExUnits>,
    #[n(22)]
    pub max_value_size: Option<u32>,
    #[n(23)]
    pub collateral_percentage: Option<u32>,
    #[n(24)]
    pub max_collateral_inputs: Option<u32>,

    #[n(25)]
    pub pool_voting_thresholds: Option<PoolVotingThresholds>,
    #[n(26)]
    pub drep_voting_thresholds: Option<DRepVotingThresholds>,
    #[n(27)]
    pub min_committee_size: Option<u32>,
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
    pub minfee_refscript_cost_per_byte: Option<Epoch>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct PoolVotingThresholds {
    pub motion_no_confidence: UnitInterval,
    pub committee_normal: UnitInterval,
    pub committee_no_confidence: UnitInterval,
    pub hard_fork_initiation: UnitInterval,
}

impl<'b, C> minicbor::Decode<'b, C> for PoolVotingThresholds {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        Ok(Self {
            motion_no_confidence: d.decode_with(ctx)?,
            committee_normal: d.decode_with(ctx)?,
            committee_no_confidence: d.decode_with(ctx)?,
            hard_fork_initiation: d.decode_with(ctx)?,
        })
    }
}

impl<C> minicbor::Encode<C> for PoolVotingThresholds {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(4)?;

        e.encode_with(&self.motion_no_confidence, ctx)?;
        e.encode_with(&self.committee_normal, ctx)?;
        e.encode_with(&self.committee_no_confidence, ctx)?;
        e.encode_with(&self.hard_fork_initiation, ctx)?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct DRepVotingThresholds {
    pub motion_no_confidence: UnitInterval,
    pub committee_normal: UnitInterval,
    pub committee_no_confidence: UnitInterval,
    pub update_constitution: UnitInterval,
    pub hard_fork_initiation: UnitInterval,
    pub pp_network_group: UnitInterval,
    pub pp_economic_group: UnitInterval,
    pub pp_technical_group: UnitInterval,
    pub pp_governance_group: UnitInterval,
    pub treasury_withdrawal: UnitInterval,
}

impl<'b, C> minicbor::Decode<'b, C> for DRepVotingThresholds {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        Ok(Self {
            motion_no_confidence: d.decode_with(ctx)?,
            committee_normal: d.decode_with(ctx)?,
            committee_no_confidence: d.decode_with(ctx)?,
            update_constitution: d.decode_with(ctx)?,
            hard_fork_initiation: d.decode_with(ctx)?,
            pp_network_group: d.decode_with(ctx)?,
            pp_economic_group: d.decode_with(ctx)?,
            pp_technical_group: d.decode_with(ctx)?,
            pp_governance_group: d.decode_with(ctx)?,
            treasury_withdrawal: d.decode_with(ctx)?,
        })
    }
}

impl<C> minicbor::Encode<C> for DRepVotingThresholds {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(10)?;

        e.encode_with(&self.motion_no_confidence, ctx)?;
        e.encode_with(&self.committee_normal, ctx)?;
        e.encode_with(&self.committee_no_confidence, ctx)?;
        e.encode_with(&self.update_constitution, ctx)?;
        e.encode_with(&self.hard_fork_initiation, ctx)?;
        e.encode_with(&self.pp_network_group, ctx)?;
        e.encode_with(&self.pp_economic_group, ctx)?;
        e.encode_with(&self.pp_technical_group, ctx)?;
        e.encode_with(&self.pp_governance_group, ctx)?;
        e.encode_with(&self.treasury_withdrawal, ctx)?;

        Ok(())
    }
}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct PseudoTransactionBody<T1> {
    #[n(0)]
    pub inputs: Set<TransactionInput>,

    #[n(1)]
    pub outputs: Vec<T1>,

    #[n(2)]
    pub fee: Coin,

    #[n(3)]
    pub ttl: Option<u64>,

    #[n(4)]
    pub certificates: Option<NonEmptySet<Certificate>>,

    #[n(5)]
    pub withdrawals: Option<KeyValuePairs<RewardAccount, Coin>>, // TODO: NON EMPTY

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
    pub collateral_return: Option<T1>,

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

pub type TransactionBody = PseudoTransactionBody<TransactionOutput>;

pub type MintedTransactionBody<'a> = PseudoTransactionBody<MintedTransactionOutput<'a>>;

impl<'a> From<MintedTransactionBody<'a>> for TransactionBody {
    fn from(value: MintedTransactionBody<'a>) -> Self {
        Self {
            inputs: value.inputs,
            outputs: value.outputs.into_iter().map(|x| x.into()).collect(),
            fee: value.fee,
            ttl: value.ttl,
            certificates: value.certificates,
            withdrawals: value.withdrawals,
            auxiliary_data_hash: value.auxiliary_data_hash,
            validity_interval_start: value.validity_interval_start,
            mint: value.mint,
            script_data_hash: value.script_data_hash,
            collateral: value.collateral,
            required_signers: value.required_signers,
            network_id: value.network_id,
            collateral_return: value.collateral_return.map(|x| x.into()),
            total_collateral: value.total_collateral,
            reference_inputs: value.reference_inputs,
            voting_procedures: value.voting_procedures,
            proposal_procedures: value.proposal_procedures,
            treasury_value: value.treasury_value,
            donation: value.donation,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Vote {
    No,
    Yes,
    Abstain,
}

impl<'b, C> minicbor::Decode<'b, C> for Vote {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        match d.u8()? {
            0 => Ok(Self::No),
            1 => Ok(Self::Yes),
            2 => Ok(Self::Abstain),
            _ => Err(minicbor::decode::Error::message(
                "invalid number for Vote kind",
            )),
        }
    }
}

impl<C> minicbor::Encode<C> for Vote {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match &self {
            Self::No => e.u8(0)?,
            Self::Yes => e.u8(1)?,
            Self::Abstain => e.u8(2)?,
        };

        Ok(())
    }
}

pub type VotingProcedures =
    NonEmptyKeyValuePairs<Voter, NonEmptyKeyValuePairs<GovActionId, VotingProcedure>>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct VotingProcedure {
    pub vote: Vote,
    pub anchor: Option<Anchor>,
}

impl<'b, C> minicbor::Decode<'b, C> for VotingProcedure {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        Ok(Self {
            vote: d.decode_with(ctx)?,
            anchor: d.decode_with(ctx)?,
        })
    }
}

impl<C> minicbor::Encode<C> for VotingProcedure {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(2)?;

        e.encode_with(&self.vote, ctx)?;
        e.encode_with(&self.anchor, ctx)?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ProposalProcedure {
    pub deposit: Coin,
    pub reward_account: RewardAccount,
    pub gov_action: GovAction,
    pub anchor: Anchor,
}

impl<'b, C> minicbor::Decode<'b, C> for ProposalProcedure {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        Ok(Self {
            deposit: d.decode_with(ctx)?,
            reward_account: d.decode_with(ctx)?,
            gov_action: d.decode_with(ctx)?,
            anchor: d.decode_with(ctx)?,
        })
    }
}

impl<C> minicbor::Encode<C> for ProposalProcedure {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(4)?;

        e.encode_with(self.deposit, ctx)?;
        e.encode_with(&self.reward_account, ctx)?;
        e.encode_with(&self.gov_action, ctx)?;
        e.encode_with(&self.anchor, ctx)?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum GovAction {
    ParameterChange(Option<GovActionId>, Box<ProtocolParamUpdate>),
    HardForkInitiation(Option<GovActionId>, Vec<ProtocolVersion>),
    TreasuryWithdrawals(KeyValuePairs<RewardAccount, Coin>),
    NoConfidence(Option<GovActionId>),
    UpdateCommittee(
        Option<GovActionId>,
        Set<CommitteeColdCredential>,
        KeyValuePairs<CommitteeColdCredential, Epoch>,
        UnitInterval,
    ),
    NewConstitution(Option<GovActionId>, Constitution),
    Information,
}

impl<'b, C> minicbor::decode::Decode<'b, C> for GovAction {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(GovAction::ParameterChange(a, b))
            }
            1 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(GovAction::HardForkInitiation(a, b))
            }
            2 => {
                let a = d.decode_with(ctx)?;
                Ok(GovAction::TreasuryWithdrawals(a))
            }
            3 => {
                let a = d.decode_with(ctx)?;
                Ok(GovAction::NoConfidence(a))
            }
            4 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                let d = d.decode_with(ctx)?;
                Ok(GovAction::UpdateCommittee(a, b, c, d))
            }
            5 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                Ok(GovAction::NewConstitution(a, b))
            }
            6 => Ok(GovAction::Information),
            _ => Err(minicbor::decode::Error::message(
                "unknown variant id for certificate",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for GovAction {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            GovAction::ParameterChange(a, b) => {
                e.array(3)?;
                e.u16(0)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            GovAction::HardForkInitiation(a, b) => {
                e.array(3)?;
                e.u16(1)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            GovAction::TreasuryWithdrawals(a) => {
                e.array(2)?;
                e.u16(2)?;
                e.encode_with(a, ctx)?;
            }
            GovAction::NoConfidence(a) => {
                e.array(2)?;
                e.u16(3)?;
                e.encode_with(a, ctx)?;
            }
            GovAction::UpdateCommittee(a, b, c, d) => {
                e.array(5)?;
                e.u16(4)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
                e.encode_with(d, ctx)?;
            }
            GovAction::NewConstitution(a, b) => {
                e.array(3)?;
                e.u16(5)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            // TODO: CDDL SAYS JUST "6", no group (array)
            GovAction::Information => {
                e.array(1)?;
                e.u16(6)?;
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct Constitution(Anchor, Option<ScriptHash>);

impl<'b, C> minicbor::Decode<'b, C> for Constitution {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        Ok(Self(d.decode_with(ctx)?, d.decode_with(ctx)?))
    }
}

impl<C> minicbor::Encode<C> for Constitution {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(2)?;

        e.encode_with(&self.0, ctx)?;
        e.encode_with(&self.1, ctx)?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum Voter {
    ConstitutionalCommitteeKey(AddrKeyhash),
    ConstitutionalCommitteeScript(ScriptHash),
    DRepKey(AddrKeyhash),
    DRepScript(ScriptHash),
    StakePoolKey(AddrKeyhash),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for Voter {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => Ok(Voter::ConstitutionalCommitteeKey(d.decode_with(ctx)?)),
            1 => Ok(Voter::ConstitutionalCommitteeScript(d.decode_with(ctx)?)),
            2 => Ok(Voter::DRepKey(d.decode_with(ctx)?)),
            3 => Ok(Voter::DRepScript(d.decode_with(ctx)?)),
            4 => Ok(Voter::StakePoolKey(d.decode_with(ctx)?)),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant id for DRep",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for Voter {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(2)?;

        match self {
            Voter::ConstitutionalCommitteeKey(h) => {
                e.encode_with(0, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            Voter::ConstitutionalCommitteeScript(h) => {
                e.encode_with(1, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            Voter::DRepKey(h) => {
                e.encode_with(2, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            Voter::DRepScript(h) => {
                e.encode_with(3, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
            Voter::StakePoolKey(h) => {
                e.encode_with(4, ctx)?;
                e.encode_with(h, ctx)?;

                Ok(())
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct Anchor(String, Hash<32>);

impl<'b, C> minicbor::Decode<'b, C> for Anchor {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        Ok(Self(d.decode_with(ctx)?, d.decode_with(ctx)?))
    }
}

impl<C> minicbor::Encode<C> for Anchor {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(2)?;

        e.encode_with(&self.0, ctx)?;
        e.encode_with(self.1, ctx)?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct GovActionId(Hash<32>, u32);

impl<'b, C> minicbor::Decode<'b, C> for GovActionId {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        Ok(Self(d.decode_with(ctx)?, d.decode_with(ctx)?))
    }
}

impl<C> minicbor::Encode<C> for GovActionId {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(2)?;

        e.encode_with(self.0, ctx)?;
        e.encode_with(self.1, ctx)?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum PseudoTransactionOutput<T> {
    Legacy(LegacyTransactionOutput),
    PostAlonzo(T),
}

impl<'b, C, T> minicbor::Decode<'b, C> for PseudoTransactionOutput<T>
where
    T: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Array | minicbor::data::Type::ArrayIndef => {
                Ok(PseudoTransactionOutput::Legacy(d.decode_with(ctx)?))
            }
            minicbor::data::Type::Map | minicbor::data::Type::MapIndef => {
                Ok(PseudoTransactionOutput::PostAlonzo(d.decode_with(ctx)?))
            }
            _ => Err(minicbor::decode::Error::message(
                "invalid type for transaction output struct",
            )),
        }
    }
}

impl<C, T> minicbor::Encode<C> for PseudoTransactionOutput<T>
where
    T: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            PseudoTransactionOutput::Legacy(x) => x.encode(e, ctx),
            PseudoTransactionOutput::PostAlonzo(x) => x.encode(e, ctx),
        }
    }
}

pub use crate::babbage::TransactionOutput;

pub use crate::babbage::MintedTransactionOutput;

pub use crate::babbage::PseudoPostAlonzoTransactionOutput;

pub use crate::babbage::PostAlonzoTransactionOutput;

pub use crate::babbage::MintedPostAlonzoTransactionOutput;

pub use crate::alonzo::VKeyWitness;

pub use crate::alonzo::NativeScript;

pub use crate::alonzo::PlutusScript as PlutusV1Script;

pub use crate::babbage::PlutusV2Script;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct PlutusV3Script(#[n(0)] pub Bytes);

impl AsRef<[u8]> for PlutusV3Script {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

pub use crate::alonzo::BigInt;

pub use crate::alonzo::PlutusData;

pub use crate::alonzo::Constr;

pub use crate::alonzo::ExUnits;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct ExUnitPrices {
    #[n(0)]
    mem_price: RationalNumber,

    #[n(1)]
    step_price: RationalNumber,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
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
    DRep,
    #[n(5)]
    VotingProposal,
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

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
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

// TODO: Redeemers needs to be KeepRaw because of script data hash
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Redeemers(NonEmptyKeyValuePairs<RedeemersKey, RedeemersValue>);

impl Deref for Redeemers {
    type Target = NonEmptyKeyValuePairs<RedeemersKey, RedeemersValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<NonEmptyKeyValuePairs<RedeemersKey, RedeemersValue>> for Redeemers {
    fn from(value: NonEmptyKeyValuePairs<RedeemersKey, RedeemersValue>) -> Self {
        Redeemers(value)
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Redeemers {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Array | minicbor::data::Type::ArrayIndef => {
                let redeemers: Vec<Redeemer> = d.decode_with(ctx)?;

                let kvs = redeemers
                    .into_iter()
                    .map(|x| {
                        (
                            RedeemersKey {
                                tag: x.tag,
                                index: x.index,
                            },
                            RedeemersValue {
                                data: x.data,
                                ex_units: x.ex_units,
                            },
                        )
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .map_err(|_| Error::message("decoding empty redeemers"))?;

                Ok(Self(kvs))
            }
            minicbor::data::Type::Map | minicbor::data::Type::MapIndef => {
                Ok(Self(d.decode_with(ctx)?))
            }
            _ => Err(minicbor::decode::Error::message(
                "invalid type for redeemers struct",
            )),
        }
    }
}

impl<C> minicbor::Encode<C> for Redeemers {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.encode_with(&self.0, ctx)?;

        Ok(())
    }
}

pub use crate::alonzo::BootstrapWitness;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct WitnessSet {
    #[n(0)]
    pub vkeywitness: Option<NonEmptySet<VKeyWitness>>,

    #[n(1)]
    pub native_script: Option<NonEmptySet<NativeScript>>,

    #[n(2)]
    pub bootstrap_witness: Option<NonEmptySet<BootstrapWitness>>,

    #[n(3)]
    pub plutus_v1_script: Option<NonEmptySet<PlutusV1Script>>,

    #[n(4)]
    pub plutus_data: Option<NonEmptySet<PlutusData>>,

    #[n(5)]
    pub redeemer: Option<Redeemers>,

    #[n(6)]
    pub plutus_v2_script: Option<NonEmptySet<PlutusV2Script>>,

    #[n(7)]
    pub plutus_v3_script: Option<NonEmptySet<PlutusV3Script>>,
}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct MintedWitnessSet<'b> {
    #[n(0)]
    pub vkeywitness: Option<NonEmptySet<VKeyWitness>>,

    #[n(1)]
    pub native_script: Option<NonEmptySet<KeepRaw<'b, NativeScript>>>,

    #[n(2)]
    pub bootstrap_witness: Option<NonEmptySet<BootstrapWitness>>,

    #[n(3)]
    pub plutus_v1_script: Option<NonEmptySet<PlutusV1Script>>,

    #[b(4)]
    pub plutus_data: Option<NonEmptySet<KeepRaw<'b, PlutusData>>>,

    #[n(5)]
    pub redeemer: Option<Redeemers>, // TODO: KeepRaw

    #[n(6)]
    pub plutus_v2_script: Option<NonEmptySet<PlutusV2Script>>,

    #[n(7)]
    pub plutus_v3_script: Option<NonEmptySet<PlutusV3Script>>,
}

impl<'b> From<MintedWitnessSet<'b>> for WitnessSet {
    fn from(x: MintedWitnessSet<'b>) -> Self {
        WitnessSet {
            vkeywitness: x.vkeywitness,
            native_script: x.native_script.map(Into::into),
            bootstrap_witness: x.bootstrap_witness,
            plutus_v1_script: x.plutus_v1_script,
            plutus_data: x.plutus_data.map(Into::into),
            redeemer: x.redeemer,
            plutus_v2_script: x.plutus_v2_script,
            plutus_v3_script: x.plutus_v3_script,
        }
    }
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct PostAlonzoAuxiliaryData {
    #[n(0)]
    pub metadata: Option<Metadata>,

    #[n(1)]
    pub native_scripts: Option<Vec<NativeScript>>,

    #[n(2)]
    pub plutus_v1_scripts: Option<Vec<PlutusV1Script>>,

    #[n(3)]
    pub plutus_v2_scripts: Option<Vec<PlutusV2Script>>,

    #[n(4)]
    pub plutus_v3_scripts: Option<Vec<PlutusV3Script>>,
}

pub use crate::babbage::DatumHash;

pub use crate::babbage::PseudoDatumOption;

pub use crate::babbage::DatumOption;

pub use crate::babbage::MintedDatumOption;

// script = [ 0, native_script // 1, plutus_v1_script // 2, plutus_v2_script ]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PseudoScript<T1> {
    NativeScript(T1),
    PlutusV1Script(PlutusV1Script),
    PlutusV2Script(PlutusV2Script),
    PlutusV3Script(PlutusV3Script),
}

// script_ref = #6.24(bytes .cbor script)
pub type ScriptRef = PseudoScript<NativeScript>;

pub type MintedScriptRef<'b> = PseudoScript<KeepRaw<'b, NativeScript>>;

impl<'b> From<MintedScriptRef<'b>> for ScriptRef {
    fn from(value: MintedScriptRef<'b>) -> Self {
        match value {
            PseudoScript::NativeScript(x) => Self::NativeScript(x.unwrap()),
            PseudoScript::PlutusV1Script(x) => Self::PlutusV1Script(x),
            PseudoScript::PlutusV2Script(x) => Self::PlutusV2Script(x),
            PseudoScript::PlutusV3Script(x) => Self::PlutusV3Script(x),
        }
    }
}

impl<'b, C, T> minicbor::Decode<'b, C> for PseudoScript<T>
where
    T: minicbor::Decode<'b, ()>,
{
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        d.array()?;

        match d.u8()? {
            0 => Ok(Self::NativeScript(d.decode()?)),
            1 => Ok(Self::PlutusV1Script(d.decode()?)),
            2 => Ok(Self::PlutusV2Script(d.decode()?)),
            3 => Ok(Self::PlutusV3Script(d.decode()?)),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant for script enum",
            )),
        }
    }
}

impl<C, T> minicbor::Encode<C> for PseudoScript<T>
where
    T: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Self::NativeScript(x) => e.encode_with((0, x), ctx)?,
            Self::PlutusV1Script(x) => e.encode_with((1, x), ctx)?,
            Self::PlutusV2Script(x) => e.encode_with((2, x), ctx)?,
            Self::PlutusV3Script(x) => e.encode_with((3, x), ctx)?,
        };

        Ok(())
    }
}

pub use crate::alonzo::Metadatum;

pub use crate::alonzo::MetadatumLabel;

pub use crate::alonzo::Metadata;

pub use crate::alonzo::AuxiliaryData;

pub use crate::alonzo::TransactionIndex;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
pub struct PseudoBlock<T1, T2, T3, T4>
where
    T4: std::clone::Clone,
{
    #[n(0)]
    pub header: T1,

    #[b(1)]
    pub transaction_bodies: MaybeIndefArray<T2>,

    #[n(2)]
    pub transaction_witness_sets: MaybeIndefArray<T3>,

    #[n(3)]
    pub auxiliary_data_set: KeyValuePairs<TransactionIndex, T4>,

    #[n(4)]
    pub invalid_transactions: Option<MaybeIndefArray<TransactionIndex>>,
}

pub type Block = PseudoBlock<Header, TransactionBody, WitnessSet, AuxiliaryData>;

/// A memory representation of an already minted block
///
/// This structure is analogous to [Block], but it allows to retrieve the
/// original CBOR bytes for each structure that might require hashing. In this
/// way, we make sure that the resulting hash matches what exists on-chain.
pub type MintedBlock<'b> = PseudoBlock<
    KeepRaw<'b, Header>,
    KeepRaw<'b, MintedTransactionBody<'b>>,
    KeepRaw<'b, MintedWitnessSet<'b>>,
    KeepRaw<'b, AuxiliaryData>,
>;

impl<'b> From<MintedBlock<'b>> for Block {
    fn from(x: MintedBlock<'b>) -> Self {
        Block {
            header: x.header.unwrap(),
            transaction_bodies: MaybeIndefArray::Def(
                x.transaction_bodies
                    .iter()
                    .cloned()
                    .map(|x| x.unwrap())
                    .map(TransactionBody::from)
                    .collect(),
            ),
            transaction_witness_sets: MaybeIndefArray::Def(
                x.transaction_witness_sets
                    .iter()
                    .cloned()
                    .map(|x| x.unwrap())
                    .map(WitnessSet::from)
                    .collect(),
            ),
            auxiliary_data_set: x
                .auxiliary_data_set
                .to_vec()
                .into_iter()
                .map(|(k, v)| (k, v.unwrap()))
                .collect::<Vec<_>>()
                .into(),
            invalid_transactions: x.invalid_transactions,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Encode, Decode, Debug)]
pub struct PseudoTx<T1, T2, T3>
where
    T1: std::clone::Clone,
    T2: std::clone::Clone,
    T3: std::clone::Clone,
{
    #[n(0)]
    pub transaction_body: T1,

    #[n(1)]
    pub transaction_witness_set: T2,

    #[n(2)]
    pub success: bool,

    #[n(3)]
    pub auxiliary_data: Nullable<T3>,
}

pub type Tx = PseudoTx<TransactionBody, WitnessSet, AuxiliaryData>;

pub type MintedTx<'b> = PseudoTx<
    KeepRaw<'b, MintedTransactionBody<'b>>,
    KeepRaw<'b, MintedWitnessSet<'b>>,
    KeepRaw<'b, AuxiliaryData>,
>;

impl<'b> From<MintedTx<'b>> for Tx {
    fn from(x: MintedTx<'b>) -> Self {
        Tx {
            transaction_body: x.transaction_body.unwrap().into(),
            transaction_witness_set: x.transaction_witness_set.unwrap().into(),
            success: x.success,
            auxiliary_data: x.auxiliary_data.map(|x| x.unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use super::MintedBlock;

    type BlockWrapper<'b> = (u16, MintedBlock<'b>);

    #[test]
    fn block_isomorphic_decoding_encoding() {
        let test_blocks = [
            include_str!("../../../test_data/conway1.block"),
            include_str!("../../../test_data/conway1.artificial.block"),
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
