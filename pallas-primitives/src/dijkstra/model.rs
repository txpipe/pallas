//! Ledger primitives and cbor codec for the Dijkstra era
//!
//! Handcrafted, idiomatic rust artifacts based on the [Dijkstra CDDL](https://github.com/IntersectMBO/cardano-ledger/blob/1587f21a7d1306dc590c2749a5c66232ef66aad0/eras/dijkstra/impl/cddl/data/dijkstra.cddl) file in the IntersectMBO repo, vendored beside this one as `defs.cddl`.

use serde::{Deserialize, Serialize};

use pallas_codec::minicbor::{self, Decode, Encode};

pub use pallas_codec::codec_by_datatype;

pub use crate::{
    AddrKeyhash, AssetName, Bytes, Coin, CostModel, DnsName, Epoch, ExUnits, GenesisDelegateHash,
    Genesishash, Hash, IPv4, IPv6, KeepRaw, MaybeIndefArray, Metadata, Metadatum, MetadatumLabel,
    NetworkId, NonZeroInt, Nonce, NonceVariant, Nullable, PlutusScript, PolicyId, PoolKeyhash,
    PoolMetadata, PoolMetadataHash, Port, PositiveCoin, PositiveInterval, ProtocolVersion,
    RationalNumber, Relay, RewardAccount, ScriptHash, StakeCredential, TransactionInput,
    UnitInterval, VrfCert, VrfKeyhash, plutus_data::*,
};

use crate::BTreeMap;

use crate::babbage;

/// Which arm of `set<a0> = #6.258([* a0])/ [* a0]` (`defs.cddl`) a value was read from.
#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Serialize, Deserialize, Hash,
)]
pub enum SetArm {
    #[default]
    Tagged,
    Bare,
}

/// Compared here so a zero minimum is a value rather than a constant comparison.
fn at_least(len: usize, minimum: usize, rule: &str) -> Result<(), minicbor::decode::Error> {
    if len < minimum {
        return Err(minicbor::decode::Error::message(format!(
            "too few elements for {rule}: {len} of at least {minimum}"
        )));
    }

    Ok(())
}

macro_rules! tag_preserving_set {
    ($name:ident, $rule:literal, $minimum:expr) => {
        impl<T> $name<T> {
            /// The arm the bytes carried, or [`SetArm::Tagged`] if this value was built.
            pub fn arm(&self) -> SetArm {
                self.arm
            }

            pub fn with_arm(self, arm: SetArm) -> Self {
                Self { arm, ..self }
            }

            pub fn into_vec(self) -> Vec<T> {
                self.items
            }
        }

        impl<T> std::ops::Deref for $name<T> {
            type Target = Vec<T>;

            fn deref(&self) -> &Self::Target {
                &self.items
            }
        }

        impl<'a, T> IntoIterator for &'a $name<T> {
            type Item = &'a T;
            type IntoIter = std::slice::Iter<'a, T>;

            fn into_iter(self) -> Self::IntoIter {
                self.items.iter()
            }
        }

        impl<T> From<$name<KeepRaw<'_, T>>> for $name<T> {
            fn from(value: $name<KeepRaw<'_, T>>) -> Self {
                Self {
                    arm: value.arm,
                    items: value.items.into_iter().map(|x| x.unwrap()).collect(),
                }
            }
        }

        impl<'b, C, T> minicbor::decode::Decode<'b, C> for $name<T>
        where
            T: Decode<'b, C>,
        {
            fn decode(
                d: &mut minicbor::Decoder<'b>,
                ctx: &mut C,
            ) -> Result<Self, minicbor::decode::Error> {
                let arm = if d.datatype()? == minicbor::data::Type::Tag {
                    let found = d.tag()?;
                    if found != minicbor::data::Tag::new(258) {
                        return Err(minicbor::decode::Error::message(format!(
                            concat!("unrecognised tag on ", $rule, ": {:?}"),
                            found
                        )));
                    }
                    SetArm::Tagged
                } else {
                    SetArm::Bare
                };

                let items: Vec<T> = d.decode_with(ctx)?;
                at_least(items.len(), $minimum, $rule)?;

                Ok(Self { arm, items })
            }
        }

        impl<C, T> minicbor::encode::Encode<C> for $name<T>
        where
            T: Encode<C>,
        {
            fn encode<W: minicbor::encode::Write>(
                &self,
                e: &mut minicbor::Encoder<W>,
                ctx: &mut C,
            ) -> Result<(), minicbor::encode::Error<W::Error>> {
                if self.arm == SetArm::Tagged {
                    e.tag(minicbor::data::Tag::new(258))?;
                }
                e.encode_with(&self.items, ctx)?;

                Ok(())
            }
        }
    };
}

/// `set<a0> = #6.258([* a0])/ [* a0]` (`defs.cddl`). Both arms are legal here,
/// so the arm is kept: a transaction id is the hash of the body bytes.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Set<T> {
    #[serde(skip)]
    arm: SetArm,
    items: Vec<T>,
}

impl<T> From<Vec<T>> for Set<T> {
    fn from(items: Vec<T>) -> Self {
        Self {
            arm: SetArm::default(),
            items,
        }
    }
}

tag_preserving_set!(Set, "a set", 0);

/// `nonempty_set<a0> = #6.258([+ a0])/ [+ a0]` (`defs.cddl`). Carries its arm like [`Set`].
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NonEmptySet<T> {
    #[serde(skip)]
    arm: SetArm,
    items: Vec<T>,
}

impl<T> NonEmptySet<T> {
    pub fn from_vec(items: Vec<T>) -> Option<Self> {
        if items.is_empty() {
            None
        } else {
            Some(Self {
                arm: SetArm::default(),
                items,
            })
        }
    }
}

impl<T> TryFrom<Vec<T>> for NonEmptySet<T> {
    type Error = Vec<T>;

    fn try_from(items: Vec<T>) -> Result<Self, Self::Error> {
        if items.is_empty() {
            Err(items)
        } else {
            Ok(Self {
                arm: SetArm::default(),
                items,
            })
        }
    }
}

tag_preserving_set!(NonEmptySet, "a nonempty set", 1);

pub use crate::babbage::OperationalCert;

/// `header_body` (`defs.cddl`) appends `block_body_contains_leios_cert` and `eb_announcement` to Conway's ten fields.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct HeaderBody {
    #[n(0)]
    pub block_number: u64,

    #[n(1)]
    pub slot: u64,

    #[n(2)]
    pub prev_hash: Option<Hash<32>>,

    #[n(3)]
    pub issuer_vkey: Bytes,

    #[n(4)]
    pub vrf_vkey: Bytes,

    #[n(5)]
    pub vrf_result: VrfCert,

    #[n(6)]
    pub block_body_size: u64,

    #[n(7)]
    pub block_body_hash: Hash<32>,

    #[n(8)]
    pub operational_cert: OperationalCert,

    #[n(9)]
    pub protocol_version: ProtocolVersion,

    // -- NEW IN DIJKSTRA
    #[n(10)]
    pub block_body_contains_leios_cert: bool,

    #[n(11)]
    pub eb_announcement: Nullable<EbAnnouncement>,
}

impl HeaderBody {
    pub fn leader_vrf_output(&self) -> Vec<u8> {
        babbage::derive_tagged_vrf_output(&self.vrf_result.0, babbage::VrfDerivation::Leader)
    }

    pub fn nonce_vrf_output(&self) -> Vec<u8> {
        babbage::derive_tagged_vrf_output(&self.vrf_result.0, babbage::VrfDerivation::Nonce)
    }
}

/// `header = [header_body, body_signature]` (`defs.cddl`).
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Header {
    #[n(0)]
    pub header_body: HeaderBody,

    #[n(1)]
    pub body_signature: Bytes,
}

pub use crate::conway::Multiasset;

pub use crate::conway::Mint;

pub use crate::conway::Value;

pub use crate::conway::Withdrawals;

pub use crate::conway::DRep;

pub use crate::conway::DRepCredential;

pub use crate::conway::CommitteeColdCredential;

pub use crate::conway::CommitteeHotCredential;

pub use crate::conway::Vote;

pub use crate::conway::VotingProcedures;

pub use crate::conway::VotingProcedure;

pub use crate::conway::Constitution;

pub use crate::conway::Voter;

pub use crate::conway::Anchor;

pub use crate::conway::GovActionId;

pub use crate::conway::PoolVotingThresholds;

pub use crate::conway::DRepVotingThresholds;

pub use crate::conway::ExUnitPrices;

pub use crate::conway::VKeyWitness;

pub use crate::conway::BootstrapWitness;

pub use crate::conway::DatumHash;

pub use crate::conway::DatumOption;

pub use crate::conway::LegacyTransactionOutput;

/// `eb_announcement = [eb_hash : hash32, eb_size : uint .size 4]` (`defs.cddl`).
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct EbAnnouncement {
    #[n(0)]
    pub eb_hash: Hash<32>,

    #[n(1)]
    pub eb_size: u32,
}

/// `bls_key = [bls_pubkey : bytes .size 96, bls_possession_proof : bytes .size 48]` (`defs.cddl`).
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct BlsKey {
    #[n(0)]
    pub bls_pubkey: Bytes,

    #[n(1)]
    pub bls_possession_proof: Bytes,
}

/// `leios_signature = bytes .size 48` (`defs.cddl`).
pub type LeiosSignature = Bytes;

/// `leios_certificate = [signers : bytes .size (0 .. 8192), signature : leios_signature]` (`defs.cddl`).
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct LeiosCertificate {
    /// Bitfield naming the signers.
    #[n(0)]
    pub signers: Bytes,

    #[n(1)]
    pub signature: LeiosSignature,
}

/// `peras_certificate = bytes` (`defs.cddl`).
pub type PerasCertificate = Bytes;

/// `certificate` (`defs.cddl`) drops Conway's `account_registration_cert` (0)
/// and `account_unregistration_cert` (1), and `pool_params` gains an optional
/// `bls_key` at position 3. The slot is absent, `nil` or populated, and all
/// three re-encode differently, which is why it is an optional nullable.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Certificate {
    StakeDelegation(StakeCredential, PoolKeyhash),
    PoolRegistration {
        operator: PoolKeyhash,
        vrf_keyhash: VrfKeyhash,
        bls_key: Option<Nullable<BlsKey>>,
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
    ResignCommitteeCold(CommitteeColdCredential, Option<Anchor>),
    RegDRepCert(DRepCredential, Coin, Option<Anchor>),
    UnRegDRepCert(DRepCredential, Coin),
    UpdateDRepCert(DRepCredential, Option<Anchor>),
}

impl<'b, C> minicbor::Decode<'b, C> for Certificate {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            2 => Ok(Certificate::StakeDelegation(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            3 => {
                let operator = d.decode_with(ctx)?;
                let vrf_keyhash = d.decode_with(ctx)?;

                // `pledge` is a coin, so a uint in this slot means the optional
                // `bls_key` was omitted, and an array or a null means it was supplied.
                let bls_key = match d.datatype()? {
                    minicbor::data::Type::Array | minicbor::data::Type::ArrayIndef => {
                        Some(Nullable::Some(d.decode_with(ctx)?))
                    }
                    minicbor::data::Type::Null => {
                        d.null()?;
                        Some(Nullable::Null)
                    }
                    _ => None,
                };

                Ok(Certificate::PoolRegistration {
                    operator,
                    vrf_keyhash,
                    bls_key,
                    pledge: d.decode_with(ctx)?,
                    cost: d.decode_with(ctx)?,
                    margin: d.decode_with(ctx)?,
                    reward_account: d.decode_with(ctx)?,
                    pool_owners: d.decode_with(ctx)?,
                    relays: d.decode_with(ctx)?,
                    pool_metadata: d.decode_with(ctx)?,
                })
            }
            4 => Ok(Certificate::PoolRetirement(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            7 => Ok(Certificate::Reg(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            8 => Ok(Certificate::UnReg(d.decode_with(ctx)?, d.decode_with(ctx)?)),
            9 => Ok(Certificate::VoteDeleg(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            10 => Ok(Certificate::StakeVoteDeleg(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            11 => Ok(Certificate::StakeRegDeleg(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            12 => Ok(Certificate::VoteRegDeleg(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            13 => Ok(Certificate::StakeVoteRegDeleg(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            14 => Ok(Certificate::AuthCommitteeHot(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            15 => Ok(Certificate::ResignCommitteeCold(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            16 => Ok(Certificate::RegDRepCert(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            17 => Ok(Certificate::UnRegDRepCert(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            18 => Ok(Certificate::UpdateDRepCert(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            0 | 1 => Err(minicbor::decode::Error::message(format!(
                "certificate variant {variant} was removed in Dijkstra"
            ))),
            _ => Err(minicbor::decode::Error::message(format!(
                "unknown certificate variant {variant}"
            ))),
        }
    }
}

impl<C> minicbor::Encode<C> for Certificate {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Certificate::StakeDelegation(a, b) => {
                e.array(3)?;
                e.encode_with(2u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::PoolRegistration {
                operator,
                vrf_keyhash,
                bls_key,
                pledge,
                cost,
                margin,
                reward_account,
                pool_owners,
                relays,
                pool_metadata,
            } => {
                e.array(if bls_key.is_some() { 11 } else { 10 })?;
                e.encode_with(3u16, ctx)?;
                e.encode_with(operator, ctx)?;
                e.encode_with(vrf_keyhash, ctx)?;
                match bls_key {
                    Some(Nullable::Some(k)) => {
                        e.encode_with(k, ctx)?;
                    }
                    Some(_) => {
                        e.null()?;
                    }
                    None => {}
                }
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
                e.encode_with(4u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::Reg(a, b) => {
                e.array(3)?;
                e.encode_with(7u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::UnReg(a, b) => {
                e.array(3)?;
                e.encode_with(8u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::VoteDeleg(a, b) => {
                e.array(3)?;
                e.encode_with(9u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::StakeVoteDeleg(a, b, c) => {
                e.array(4)?;
                e.encode_with(10u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::StakeRegDeleg(a, b, c) => {
                e.array(4)?;
                e.encode_with(11u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::VoteRegDeleg(a, b, c) => {
                e.array(4)?;
                e.encode_with(12u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::StakeVoteRegDeleg(a, b, c, dd) => {
                e.array(5)?;
                e.encode_with(13u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
                e.encode_with(dd, ctx)?;
            }
            Certificate::AuthCommitteeHot(a, b) => {
                e.array(3)?;
                e.encode_with(14u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::ResignCommitteeCold(a, b) => {
                e.array(3)?;
                e.encode_with(15u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::RegDRepCert(a, b, c) => {
                e.array(4)?;
                e.encode_with(16u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;
            }
            Certificate::UnRegDRepCert(a, b) => {
                e.array(3)?;
                e.encode_with(17u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            Certificate::UpdateDRepCert(a, b) => {
                e.array(3)?;
                e.encode_with(18u16, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
        }

        Ok(())
    }
}

/// `language = 0 .. 3` (`defs.cddl`), widened from Conway's `0 .. 2`.
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

    #[n(3)]
    PlutusV4,
}

/// `cost_models` (`defs.cddl`) names key 3 for PlutusV4, leaving Conway's wildcard at `4 .. 255`.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct CostModels {
    pub plutus_v1: Option<CostModel>,

    pub plutus_v2: Option<CostModel>,

    pub plutus_v3: Option<CostModel>,

    pub plutus_v4: Option<CostModel>,

    /// Cost models under the keys the CDDL wildcard permits, 4 to 255.
    pub unknown: BTreeMap<u64, CostModel>,
}

impl CostModels {
    /// Every key to write, ascending, a named field before the wildcard.
    fn entries(&self) -> BTreeMap<u64, &CostModel> {
        let mut entries: BTreeMap<u64, &CostModel> =
            self.unknown.iter().map(|(k, v)| (*k, v)).collect();

        let named = [
            (0u64, &self.plutus_v1),
            (1, &self.plutus_v2),
            (2, &self.plutus_v3),
            (3, &self.plutus_v4),
        ];

        for (key, model) in named {
            if let Some(model) = model {
                entries.insert(key, model);
            }
        }

        entries
    }
}

impl<C> minicbor::Encode<C> for CostModels {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let entries = self.entries();

        e.map(entries.len() as u64)?;
        for (key, model) in entries {
            e.u64(key)?;
            e.encode_with(model, ctx)?;
        }

        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for CostModels {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let models: BTreeMap<u64, CostModel> = d.decode_with(ctx)?;

        let mut plutus_v1 = None;
        let mut plutus_v2 = None;
        let mut plutus_v3 = None;
        let mut plutus_v4 = None;
        let mut unknown: Vec<(u64, CostModel)> = Vec::new();

        for (k, v) in models.iter() {
            match k {
                0 => plutus_v1 = Some(v.clone()),
                1 => plutus_v2 = Some(v.clone()),
                2 => plutus_v3 = Some(v.clone()),
                3 => plutus_v4 = Some(v.clone()),
                _ => unknown.push((*k, v.clone())),
            }
        }

        Ok(Self {
            plutus_v1,
            plutus_v2,
            plutus_v3,
            plutus_v4,
            unknown: unknown.into_iter().collect(),
        })
    }
}

/// `protocol_param_update` (`defs.cddl`) gains keys 34 through 48. A derived
/// map decoder drops a key it has no field for, so the tail is modelled.
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

    // -- NEW IN DIJKSTRA
    #[n(34)]
    pub max_ref_script_size_per_block: Option<u64>,
    #[n(35)]
    pub max_ref_script_size_per_tx: Option<u64>,
    #[n(36)]
    pub ref_script_cost_stride: Option<u64>,
    #[n(37)]
    pub ref_script_cost_multiplier: Option<PositiveInterval>,

    /// `max_pledge_leverage = nonnegative_interval/ nil` (`defs.cddl`), so an explicit nil differs from an absent key.
    #[n(38)]
    pub max_pledge_leverage: Option<Nullable<RationalNumber>>,
    #[n(39)]
    pub min_pool_margin: Option<UnitInterval>,
    #[n(40)]
    pub leios_announcement_period_length: Option<u64>,
    #[n(41)]
    pub leios_vote_period_length: Option<u64>,
    #[n(42)]
    pub leios_diffusion_period_length: Option<u64>,
    #[n(43)]
    pub leios_committee_size: Option<u64>,
    #[n(44)]
    pub leios_quorum_stake_threshold: Option<UnitInterval>,
    #[n(45)]
    pub max_endorser_block_references_size: Option<u64>,
    #[n(46)]
    pub max_endorser_block_txs_size: Option<u64>,
    #[n(47)]
    pub max_endorser_block_execution_units: Option<ExUnits>,
    #[n(48)]
    pub max_ref_script_size_per_endorser_block: Option<u64>,
}

/// No CDDL rule answers to this type. It mirrors [`crate::conway::Update`] over this era's `protocol_param_update`.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Update {
    #[n(0)]
    pub proposed_protocol_parameter_updates: BTreeMap<Genesishash, ProtocolParamUpdate>,

    #[n(1)]
    pub epoch: Epoch,
}

/// `proposal_procedure = [deposit : coin, reward_account, gov_action, anchor]` (`defs.cddl`).
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
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

/// `gov_action` (`defs.cddl`). Its `parameter_change_action` arm reaches this era's `protocol_param_update`.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
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

/// `guards = nonempty_set<addr_keyhash>/ nonempty_oset<credential>`
/// (`defs.cddl`), Conway's `required_signers` widened. The arms are told
/// apart by element type: an `addr_keyhash` is a byte string, a `credential`
/// an array.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Guards {
    AddrKeyhashes(NonEmptySet<AddrKeyhash>),
    Credentials(NonEmptySet<StakeCredential>),
}

fn empty_guards() -> minicbor::decode::Error {
    minicbor::decode::Error::message("guards must carry at least one element")
}

impl<'b, C> minicbor::Decode<'b, C> for Guards {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let mut probe = minicbor::Decoder::new(&d.input()[d.position()..]);
        if probe.datatype()? == minicbor::data::Type::Tag {
            probe.tag()?;
        }
        let len = probe.array()?;

        // Reading the element type of an empty array would read the byte after
        // the value, which belongs to whatever encloses it.
        if len == Some(0) {
            return Err(empty_guards());
        }

        match probe.datatype()? {
            minicbor::data::Type::Break => Err(empty_guards()),
            minicbor::data::Type::Array | minicbor::data::Type::ArrayIndef => {
                Ok(Guards::Credentials(d.decode_with(ctx)?))
            }
            minicbor::data::Type::Bytes | minicbor::data::Type::BytesIndef => {
                Ok(Guards::AddrKeyhashes(d.decode_with(ctx)?))
            }
            other => Err(minicbor::decode::Error::message(format!(
                "invalid guards element type {other} at position {}",
                d.position()
            ))),
        }
    }
}

impl<C> minicbor::Encode<C> for Guards {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Guards::AddrKeyhashes(x) => e.encode_with(x, ctx)?,
            Guards::Credentials(x) => e.encode_with(x, ctx)?,
        };

        Ok(())
    }
}

pub type DirectDeposits = BTreeMap<RewardAccount, Coin>;

pub type RequiredTopLevelGuards = BTreeMap<StakeCredential, Nullable<PlutusData>>;

pub type AccountBalanceIntervals = BTreeMap<RewardAccount, AccountBalanceInterval>;

pub type StartingAccountBalanceIntervals = BTreeMap<RewardAccount, AccountBalanceInterval>;

/// `account_balance_interval` (`defs.cddl`), a two element array with at most
/// one bound missing, or a bare `coin`.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum AccountBalanceInterval {
    /// `[coin, nil]`
    LowerBound(Coin),
    /// `[coin, coin]`
    Bounded(Coin, Coin),
    /// `[nil, coin]`
    UpperBound(Coin),
    /// A bare `coin`.
    Exact(Coin),
}

impl<'b, C> minicbor::Decode<'b, C> for AccountBalanceInterval {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Array | minicbor::data::Type::ArrayIndef => {}
            _ => return Ok(AccountBalanceInterval::Exact(d.decode_with(ctx)?)),
        }

        let len = d.array()?;
        match len {
            None | Some(2) => {}
            Some(found) => {
                return Err(minicbor::decode::Error::message(format!(
                    "account_balance_interval takes two elements, found {found}"
                )));
            }
        }

        let lower: Option<Coin> = match d.datatype()? {
            minicbor::data::Type::Null => {
                d.null()?;
                None
            }
            _ => Some(d.decode_with(ctx)?),
        };

        let upper: Option<Coin> = match d.datatype()? {
            minicbor::data::Type::Null => {
                d.null()?;
                None
            }
            _ => Some(d.decode_with(ctx)?),
        };

        if len.is_none() {
            if d.datatype()? != minicbor::data::Type::Break {
                return Err(minicbor::decode::Error::message(
                    "account_balance_interval takes two elements, found more",
                ));
            }
            d.skip()?;
        }

        match (lower, upper) {
            (Some(l), None) => Ok(AccountBalanceInterval::LowerBound(l)),
            (Some(l), Some(u)) => Ok(AccountBalanceInterval::Bounded(l, u)),
            (None, Some(u)) => Ok(AccountBalanceInterval::UpperBound(u)),
            (None, None) => Err(minicbor::decode::Error::message(
                "account_balance_interval with neither a lower nor an upper bound",
            )),
        }
    }
}

impl<C> minicbor::Encode<C> for AccountBalanceInterval {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            AccountBalanceInterval::LowerBound(l) => {
                e.array(2)?;
                e.encode_with(l, ctx)?;
                e.null()?;
            }
            AccountBalanceInterval::Bounded(l, u) => {
                e.array(2)?;
                e.encode_with(l, ctx)?;
                e.encode_with(u, ctx)?;
            }
            AccountBalanceInterval::UpperBound(u) => {
                e.array(2)?;
                e.null()?;
                e.encode_with(u, ctx)?;
            }
            AccountBalanceInterval::Exact(c) => {
                e.encode_with(c, ctx)?;
            }
        }

        Ok(())
    }
}

/// `sub_transaction_body` (`defs.cddl`), Conway's body minus the keys a sub
/// transaction cannot carry, such as the fee and the collateral.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct SubTransactionBody<'a> {
    #[n(0)]
    pub inputs: Set<TransactionInput>,

    #[b(1)]
    pub outputs: MaybeIndefArray<TransactionOutput<'a>>,

    #[n(3)]
    pub ttl: Option<u64>,

    #[n(4)]
    pub certificates: Option<NonEmptySet<Certificate>>,

    #[n(5)]
    pub withdrawals: Option<Withdrawals>,

    #[n(7)]
    pub auxiliary_data_hash: Option<Hash<32>>,

    #[n(8)]
    pub validity_interval_start: Option<u64>,

    #[n(9)]
    pub mint: Option<Multiasset<NonZeroInt>>,

    #[n(11)]
    pub script_data_hash: Option<Hash<32>>,

    #[n(14)]
    pub guards: Option<Guards>,

    #[n(15)]
    pub network_id: Option<NetworkId>,

    #[n(18)]
    pub reference_inputs: Option<NonEmptySet<TransactionInput>>,

    #[n(19)]
    pub voting_procedures: Option<VotingProcedures>,

    #[n(20)]
    pub proposal_procedures: Option<NonEmptySet<ProposalProcedure>>,

    #[n(21)]
    pub treasury_value: Option<Coin>,

    #[n(22)]
    pub donation: Option<PositiveCoin>,

    #[n(24)]
    pub required_top_level_guards: Option<RequiredTopLevelGuards>,

    #[n(25)]
    pub direct_deposits: Option<DirectDeposits>,

    #[n(26)]
    pub account_balance_intervals: Option<AccountBalanceIntervals>,
}

/// `sub_transaction = [sub_transaction_body, transaction_witness_set, auxiliary_data/ nil]` (`defs.cddl`).
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
pub struct SubTransaction<'b> {
    #[b(0)]
    pub sub_transaction_body: KeepRaw<'b, SubTransactionBody<'b>>,

    #[n(1)]
    pub transaction_witness_set: KeepRaw<'b, WitnessSet<'b>>,

    #[n(2)]
    pub auxiliary_data: Nullable<KeepRaw<'b, AuxiliaryData>>,
}

/// `sub_transactions = nonempty_oset<sub_transaction>` (`defs.cddl`).
pub type SubTransactions<'b> = NonEmptySet<SubTransaction<'b>>;

/// `transaction_body` (`defs.cddl`). Key 14 is `guards`, not Conway's `required_signers`.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct TransactionBody<'a> {
    #[n(0)]
    pub inputs: Set<TransactionInput>,

    #[b(1)]
    pub outputs: MaybeIndefArray<TransactionOutput<'a>>,

    #[n(2)]
    pub fee: Coin,

    #[n(3)]
    pub ttl: Option<u64>,

    #[n(4)]
    pub certificates: Option<NonEmptySet<Certificate>>,

    #[n(5)]
    pub withdrawals: Option<Withdrawals>,

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
    pub guards: Option<Guards>,

    #[n(15)]
    pub network_id: Option<NetworkId>,

    #[n(16)]
    pub collateral_return: Option<TransactionOutput<'a>>,

    #[n(17)]
    pub total_collateral: Option<Coin>,

    #[n(18)]
    pub reference_inputs: Option<NonEmptySet<TransactionInput>>,

    #[n(19)]
    pub voting_procedures: Option<VotingProcedures>,

    #[n(20)]
    pub proposal_procedures: Option<NonEmptySet<ProposalProcedure>>,

    #[n(21)]
    pub treasury_value: Option<Coin>,

    #[n(22)]
    pub donation: Option<PositiveCoin>,

    // -- NEW IN DIJKSTRA
    #[b(23)]
    pub sub_transactions: Option<SubTransactions<'a>>,

    #[n(24)]
    pub required_top_level_guards: Option<RequiredTopLevelGuards>,

    #[n(25)]
    pub direct_deposits: Option<DirectDeposits>,

    #[n(26)]
    pub account_balance_intervals: Option<AccountBalanceIntervals>,

    #[n(27)]
    pub starting_account_balance_intervals: Option<StartingAccountBalanceIntervals>,
}

pub type PostAlonzoTransactionOutput<'b> =
    babbage::GenPostAlonzoTransactionOutput<'b, Value, ScriptRef<'b>>;

pub type TransactionOutput<'b> = babbage::GenTransactionOutput<'b, PostAlonzoTransactionOutput<'b>>;

// FIXME: Repeated since macro does not handle type generics yet.
codec_by_datatype! {
    TransactionOutput<'b>,
    Array | ArrayIndef => Legacy,
    Map | MapIndef => PostAlonzo,
    ()
}

/// `native_script` gains `script_require_guard = (6, credential)` (`defs.cddl`).
#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cbor(flat)]
pub enum NativeScript {
    #[n(0)]
    ScriptPubkey(#[n(0)] AddrKeyhash),
    #[n(1)]
    ScriptAll(#[n(0)] Vec<NativeScript>),
    #[n(2)]
    ScriptAny(#[n(0)] Vec<NativeScript>),
    #[n(3)]
    ScriptNOfK(#[n(0)] u32, #[n(1)] Vec<NativeScript>),
    #[n(4)]
    InvalidBefore(#[n(0)] u64),
    #[n(5)]
    InvalidHereafter(#[n(0)] u64),

    // -- NEW IN DIJKSTRA
    #[n(6)]
    ScriptRequireGuard(#[n(0)] StakeCredential),
}

/// `redeemer_tag` gains tag 6, `guarding` (`defs.cddl`).
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

    // -- NEW IN DIJKSTRA
    #[n(6)]
    Guarding,
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

/// `redeemers` is map only in Dijkstra (`defs.cddl`). Conway's array arm and
/// the `redeemer` rule behind it were deleted.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct Redeemers(pub BTreeMap<RedeemersKey, RedeemersValue>);

impl From<BTreeMap<RedeemersKey, RedeemersValue>> for Redeemers {
    fn from(value: BTreeMap<RedeemersKey, RedeemersValue>) -> Self {
        Redeemers(value)
    }
}

impl std::ops::Deref for Redeemers {
    type Target = BTreeMap<RedeemersKey, RedeemersValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// `transaction_witness_set` (`defs.cddl`). The rule text is Conway's, but the
/// `native_script` and `redeemers` it reaches both changed.
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

/// `auxiliary_data_map` gains `? 5 : [* plutus_v4_script]` (`defs.cddl`).
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone, Eq)]
#[cbor(map, tag(259))]
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

    // -- NEW IN DIJKSTRA
    #[n(5)]
    pub plutus_v4_scripts: Option<Vec<PlutusScript<4>>>,
}

/// `auxiliary_data_array` (`defs.cddl`), whose second element is
/// `auxiliary_scripts = [* native_script]`, this era's.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone, Eq)]
pub struct ShelleyMaAuxiliaryData {
    #[n(0)]
    pub transaction_metadata: Metadata,

    #[n(1)]
    pub auxiliary_scripts: Option<Vec<NativeScript>>,
}

/// `auxiliary_data` (`defs.cddl`).
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq)]
pub enum AuxiliaryData {
    Shelley(Metadata),
    ShelleyMa(ShelleyMaAuxiliaryData),
    PostAlonzo(PostAlonzoAuxiliaryData),
}

codec_by_datatype! {
    AuxiliaryData,
    Map | MapIndef => Shelley,
    Array | ArrayIndef => ShelleyMa,
    Tag => PostAlonzo,
    ()
}

/// `script` gains `// 4, plutus_v4_script` (`defs.cddl`), reached through
/// `script_ref = #6.24(bytes .cbor script)`.
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

    // -- NEW IN DIJKSTRA
    #[n(4)]
    PlutusV4Script(#[n(0)] PlutusScript<4>),
}

/// `block_body` (`defs.cddl`). Dijkstra replaces Conway's segregated witness
/// layout with complete inline transactions, and drops `invalid_transactions`
/// and `transaction_index` because each transaction carries its own flag.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
pub struct BlockBody<'b> {
    #[b(0)]
    pub transactions: MaybeIndefArray<BlockTransaction<'b>>,

    #[n(1)]
    pub leios_certificate: Nullable<LeiosCertificate>,

    #[n(2)]
    pub peras_certificate: Nullable<PerasCertificate>,
}

/// `block = [header, block_body]` (`defs.cddl`).
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
pub struct Block<'b> {
    #[n(0)]
    pub header: KeepRaw<'b, Header>,

    #[b(1)]
    pub block_body: BlockBody<'b>,
}

/// `block_transaction = [transaction_body, transaction_witness_set, auxiliary_data/ nil, bool]`
/// (`defs.cddl`), the four element form a ranking block carries. Conway puts
/// the flag at position 2, here it is last and is named `success` as in Conway.
#[derive(Clone, Serialize, Deserialize, Encode, Decode, Debug, PartialEq)]
pub struct BlockTransaction<'b> {
    #[b(0)]
    pub transaction_body: KeepRaw<'b, TransactionBody<'b>>,

    #[n(1)]
    pub transaction_witness_set: KeepRaw<'b, WitnessSet<'b>>,

    #[n(2)]
    pub auxiliary_data: Nullable<KeepRaw<'b, AuxiliaryData>>,

    /// Set by the block producer, not by the transaction author.
    #[n(3)]
    pub success: bool,
}

impl Eq for BlockTransaction<'_> {}

impl<'b> BlockTransaction<'b> {
    pub fn to_mempool_transaction(&self) -> MempoolTransaction<'b> {
        MempoolTransaction {
            transaction_body: self.transaction_body.clone(),
            transaction_witness_set: self.transaction_witness_set.clone(),
            is_valid_supplied: false,
            auxiliary_data: self.auxiliary_data.clone(),
        }
    }
}

/// `mempool_transaction` (`defs.cddl`) is three elements, and tolerates the
/// Conway four element shape with `is_valid` required to be `true`.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct MempoolTransaction<'b> {
    pub transaction_body: KeepRaw<'b, TransactionBody<'b>>,
    pub transaction_witness_set: KeepRaw<'b, WitnessSet<'b>>,
    pub is_valid_supplied: bool,
    pub auxiliary_data: Nullable<KeepRaw<'b, AuxiliaryData>>,
}

impl<'b, C> minicbor::Decode<'b, C> for MempoolTransaction<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let len = d.array()?;

        let transaction_body = d.decode_with(ctx)?;
        let transaction_witness_set = d.decode_with(ctx)?;

        let is_valid_supplied = match d.datatype()? {
            minicbor::data::Type::Bool => {
                if d.bool()? {
                    true
                } else {
                    return Err(minicbor::decode::Error::message(
                        "value `false` not allowed for `is_valid`",
                    ));
                }
            }
            _ => false,
        };

        let expected = if is_valid_supplied { 4 } else { 3 };

        if let Some(len) = len
            && len != expected
        {
            return Err(minicbor::decode::Error::message(format!(
                "expected a {expected} element transaction, found {len}"
            )));
        }

        Ok(MempoolTransaction {
            transaction_body,
            transaction_witness_set,
            is_valid_supplied,
            auxiliary_data: d.decode_with(ctx)?,
        })
    }
}

impl<C> minicbor::Encode<C> for MempoolTransaction<'_> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(if self.is_valid_supplied { 4 } else { 3 })?;
        e.encode_with(&self.transaction_body, ctx)?;
        e.encode_with(&self.transaction_witness_set, ctx)?;
        if self.is_valid_supplied {
            e.bool(true)?;
        }
        e.encode_with(&self.auxiliary_data, ctx)?;

        Ok(())
    }
}

impl Eq for MempoolTransaction<'_> {}
