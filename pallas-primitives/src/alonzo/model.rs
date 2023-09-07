//! Ledger primitives and cbor codec for the Alonzo era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Alonzo CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/alonzo/test-suite/cddl-files/alonzo.cddl) file in IOHK repo.

use serde::{Deserialize, Serialize};
use std::{fmt, ops::Deref};

use pallas_codec::minicbor::{data::Tag, Decode, Encode};
use pallas_crypto::hash::Hash;

use pallas_codec::utils::{Bytes, Int, KeepRaw, KeyValuePairs, MaybeIndefArray, Nullable};

// required for derive attrs to work
use pallas_codec::minicbor;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct VrfCert(#[n(0)] pub Bytes, #[n(1)] pub Bytes);

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
    pub nonce_vrf: VrfCert,

    #[n(6)]
    pub leader_vrf: VrfCert,

    #[n(7)]
    pub block_body_size: u64,

    #[n(8)]
    pub block_body_hash: Hash<32>,

    #[n(9)]
    pub operational_cert_hot_vkey: Bytes,

    #[n(10)]
    pub operational_cert_sequence_number: u64,

    #[n(11)]
    pub operational_cert_kes_period: u64,

    #[n(12)]
    pub operational_cert_sigma: Bytes,

    #[n(13)]
    pub protocol_major: u64,

    #[n(14)]
    pub protocol_minor: u64,
}

pub type ProtocolVersion = (u64, u64);

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq)]
pub struct KesSignature {}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Header {
    #[n(0)]
    pub header_body: HeaderBody,

    #[n(1)]
    pub body_signature: Bytes,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct TransactionInput {
    #[n(0)]
    pub transaction_id: Hash<32>,

    #[n(1)]
    pub index: u64,
}

// $nonce /= [ 0 // 1, bytes .size 32 ]

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(index_only)]
pub enum NonceVariant {
    #[n(0)]
    NeutralNonce,

    #[n(1)]
    Nonce,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Nonce {
    #[n(0)]
    pub variant: NonceVariant,

    #[n(1)]
    pub hash: Option<Hash<32>>,
}

pub type ScriptHash = Bytes;

pub type PolicyId = Hash<28>;

pub type AssetName = Bytes;

pub type Multiasset<A> = KeyValuePairs<PolicyId, KeyValuePairs<AssetName, A>>;

pub type Mint = Multiasset<i64>;

pub type Coin = u64;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Value {
    Coin(Coin),
    Multiasset(Coin, Multiasset<Coin>),
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

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct TransactionOutput {
    #[n(0)]
    pub address: Bytes,

    #[n(1)]
    pub amount: Value,

    #[n(2)]
    pub datum_hash: Option<Hash<32>>,
}

pub type PoolKeyhash = Hash<28>;
pub type Epoch = u64;
pub type Genesishash = Bytes;
pub type GenesisDelegateHash = Bytes;
pub type VrfKeyhash = Hash<32>;

/* move_instantaneous_reward = [ 0 / 1, { * stake_credential => delta_coin } / coin ]
; The first field determines where the funds are drawn from.
; 0 denotes the reserves, 1 denotes the treasury.
; If the second field is a map, funds are moved to stake credentials,
; otherwise the funds are given to the other accounting pot.
 */

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum InstantaneousRewardSource {
    Reserves,
    Treasury,
}

impl<'b, C> minicbor::decode::Decode<'b, C> for InstantaneousRewardSource {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let variant = d.u32()?;

        match variant {
            0 => Ok(Self::Reserves),
            1 => Ok(Self::Treasury),
            _ => Err(minicbor::decode::Error::message("invalid funds variant")),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for InstantaneousRewardSource {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let variant = match self {
            Self::Reserves => 0,
            Self::Treasury => 1,
        };

        e.u32(variant)?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum InstantaneousRewardTarget {
    StakeCredentials(KeyValuePairs<StakeCredential, i64>),
    OtherAccountingPot(Coin),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for InstantaneousRewardTarget {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let datatype = d.datatype()?;

        match datatype {
            minicbor::data::Type::Map | minicbor::data::Type::MapIndef => {
                let a = d.decode_with(ctx)?;
                Ok(Self::StakeCredentials(a))
            }
            _ => {
                let a = d.decode_with(ctx)?;
                Ok(Self::OtherAccountingPot(a))
            }
        }
    }
}

impl<C> minicbor::encode::Encode<C> for InstantaneousRewardTarget {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            InstantaneousRewardTarget::StakeCredentials(a) => {
                e.encode_with(a, ctx)?;
                Ok(())
            }
            InstantaneousRewardTarget::OtherAccountingPot(a) => {
                e.encode_with(a, ctx)?;
                Ok(())
            }
        }
    }
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cbor]
pub struct MoveInstantaneousReward {
    #[n(0)]
    pub source: InstantaneousRewardSource,

    #[n(1)]
    pub target: InstantaneousRewardTarget,
}

pub type RewardAccount = Bytes;

pub type Withdrawals = KeyValuePairs<RewardAccount, Coin>;

pub type RequiredSigners = Vec<AddrKeyhash>;

pub type Port = u32;
pub type IPv4 = Bytes;
pub type IPv6 = Bytes;
pub type DnsName = String;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Relay {
    SingleHostAddr(Option<Port>, Option<IPv4>, Option<IPv6>),
    SingleHostName(Option<Port>, DnsName),
    MultiHostName(DnsName),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for Relay {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => Ok(Relay::SingleHostAddr(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            1 => Ok(Relay::SingleHostName(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            2 => Ok(Relay::MultiHostName(d.decode_with(ctx)?)),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant id for Relay",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for Relay {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Relay::SingleHostAddr(a, b, c) => {
                e.array(4)?;
                e.encode_with(0, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;

                Ok(())
            }
            Relay::SingleHostName(a, b) => {
                e.array(3)?;
                e.encode_with(1, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;

                Ok(())
            }
            Relay::MultiHostName(a) => {
                e.array(2)?;
                e.encode_with(2, ctx)?;
                e.encode_with(a, ctx)?;

                Ok(())
            }
        }
    }
}

pub type PoolMetadataHash = Hash<32>;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct PoolMetadata {
    #[n(0)]
    pub url: String,

    #[n(1)]
    pub hash: PoolMetadataHash,
}

pub type AddrKeyhash = Hash<28>;
pub type Scripthash = Hash<28>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct RationalNumber {
    pub numerator: u64,
    pub denominator: u64,
}

impl<'b, C> minicbor::decode::Decode<'b, C> for RationalNumber {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.tag()?;
        d.array()?;

        Ok(RationalNumber {
            numerator: d.decode_with(ctx)?,
            denominator: d.decode_with(ctx)?,
        })
    }
}

impl<C> minicbor::encode::Encode<C> for RationalNumber {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        // TODO: check if this is the correct tag
        e.tag(Tag::Unassigned(30))?;
        e.array(2)?;
        e.encode_with(self.numerator, ctx)?;
        e.encode_with(self.denominator, ctx)?;

        Ok(())
    }
}

pub type UnitInterval = RationalNumber;

pub type PositiveInterval = RationalNumber;

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum StakeCredential {
    AddrKeyhash(AddrKeyhash),
    Scripthash(Scripthash),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for StakeCredential {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => Ok(StakeCredential::AddrKeyhash(d.decode_with(ctx)?)),
            1 => Ok(StakeCredential::Scripthash(d.decode_with(ctx)?)),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant id for StakeCredential",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for StakeCredential {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            StakeCredential::AddrKeyhash(a) => {
                e.array(2)?;
                e.encode_with(0, ctx)?;
                e.encode_with(a, ctx)?;

                Ok(())
            }
            StakeCredential::Scripthash(a) => {
                e.array(2)?;
                e.encode_with(1, ctx)?;
                e.encode_with(a, ctx)?;

                Ok(())
            }
        }
    }
}

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
        pool_owners: Vec<AddrKeyhash>,
        relays: Vec<Relay>,
        pool_metadata: Option<PoolMetadata>,
    },
    PoolRetirement(PoolKeyhash, Epoch),
    GenesisKeyDelegation(Genesishash, GenesisDelegateHash, VrfKeyhash),
    MoveInstantaneousRewardsCert(MoveInstantaneousReward),
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
            5 => {
                let a = d.decode_with(ctx)?;
                let b = d.decode_with(ctx)?;
                let c = d.decode_with(ctx)?;
                Ok(Certificate::GenesisKeyDelegation(a, b, c))
            }
            6 => {
                let a = d.decode_with(ctx)?;
                Ok(Certificate::MoveInstantaneousRewardsCert(a))
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

                Ok(())
            }
            Certificate::StakeDeregistration(a) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode_with(a, ctx)?;

                Ok(())
            }
            Certificate::StakeDelegation(a, b) => {
                e.array(3)?;
                e.u16(2)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;

                Ok(())
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

                Ok(())
            }
            Certificate::PoolRetirement(a, b) => {
                e.array(3)?;
                e.u16(4)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;

                Ok(())
            }
            Certificate::GenesisKeyDelegation(a, b, c) => {
                e.array(4)?;
                e.u16(5)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
                e.encode_with(c, ctx)?;

                Ok(())
            }
            Certificate::MoveInstantaneousRewardsCert(a) => {
                e.array(2)?;
                e.u16(6)?;
                e.encode_with(a, ctx)?;

                Ok(())
            }
        }
    }
}

#[derive(
    Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy,
)]
#[cbor(index_only)]
pub enum NetworkId {
    #[n(0)]
    One,
    #[n(1)]
    Two,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cbor(index_only)]
pub enum Language {
    #[n(0)]
    PlutusV1,
}

pub type CostModel = Vec<i64>;

pub type CostMdls = KeyValuePairs<Language, CostModel>;

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
    #[n(12)]
    pub decentralization_constant: Option<UnitInterval>,
    #[n(13)]
    pub extra_entropy: Option<Nonce>,
    #[n(14)]
    pub protocol_version: Option<ProtocolVersion>,
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
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Update {
    #[n(0)]
    pub proposed_protocol_parameter_updates: KeyValuePairs<Genesishash, ProtocolParamUpdate>,

    #[n(1)]
    pub epoch: Epoch,
}

// Can't derive encode for TransactionBody because it seems to require a very
// particular order for each key in the map
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(map)]
pub struct TransactionBody {
    #[n(0)]
    pub inputs: Vec<TransactionInput>,

    #[n(1)]
    pub outputs: Vec<TransactionOutput>,

    #[n(2)]
    pub fee: u64,

    #[n(3)]
    pub ttl: Option<u64>,

    #[n(4)]
    pub certificates: Option<Vec<Certificate>>,

    #[n(5)]
    pub withdrawals: Option<Withdrawals>,

    #[n(6)]
    pub update: Option<Update>,

    #[n(7)]
    pub auxiliary_data_hash: Option<Bytes>,

    #[n(8)]
    pub validity_interval_start: Option<u64>,

    #[n(9)]
    pub mint: Option<Multiasset<i64>>,

    #[n(11)]
    pub script_data_hash: Option<Hash<32>>,

    #[n(13)]
    pub collateral: Option<Vec<TransactionInput>>,

    #[n(14)]
    pub required_signers: Option<RequiredSigners>,

    #[n(15)]
    pub network_id: Option<NetworkId>,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct VKeyWitness {
    #[n(0)]
    pub vkey: Bytes,

    #[n(1)]
    pub signature: Bytes,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum NativeScript {
    ScriptPubkey(AddrKeyhash),
    ScriptAll(Vec<NativeScript>),
    ScriptAny(Vec<NativeScript>),
    ScriptNOfK(u32, Vec<NativeScript>),
    InvalidBefore(u64),
    InvalidHereafter(u64),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for NativeScript {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u32()?;

        match variant {
            0 => Ok(NativeScript::ScriptPubkey(d.decode_with(ctx)?)),
            1 => Ok(NativeScript::ScriptAll(d.decode_with(ctx)?)),
            2 => Ok(NativeScript::ScriptAny(d.decode_with(ctx)?)),
            3 => Ok(NativeScript::ScriptNOfK(
                d.decode_with(ctx)?,
                d.decode_with(ctx)?,
            )),
            4 => Ok(NativeScript::InvalidBefore(d.decode_with(ctx)?)),
            5 => Ok(NativeScript::InvalidHereafter(d.decode_with(ctx)?)),
            _ => Err(minicbor::decode::Error::message(
                "unknown variant id for native script",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for NativeScript {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(2)?;

        match self {
            NativeScript::ScriptPubkey(v) => {
                e.encode_with(0, ctx)?;
                e.encode_with(v, ctx)?;
            }
            NativeScript::ScriptAll(v) => {
                e.encode_with(1, ctx)?;
                e.encode_with(v, ctx)?;
            }
            NativeScript::ScriptAny(v) => {
                e.encode_with(2, ctx)?;
                e.encode_with(v, ctx)?;
            }
            NativeScript::ScriptNOfK(a, b) => {
                e.encode_with(3, ctx)?;
                e.encode_with(a, ctx)?;
                e.encode_with(b, ctx)?;
            }
            NativeScript::InvalidBefore(v) => {
                e.encode_with(4, ctx)?;
                e.encode_with(v, ctx)?;
            }
            NativeScript::InvalidHereafter(v) => {
                e.encode_with(5, ctx)?;
                e.encode_with(v, ctx)?;
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct PlutusScript(#[n(0)] pub Bytes);

impl AsRef<[u8]> for PlutusScript {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

/// Defined to encode PlutusData bytestring as it is done in the canonical
/// plutus implementation
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(into = "String")]
#[serde(try_from = "String")]
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

impl fmt::Display for BoundedBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes: Vec<u8> = self.clone().into();

        f.write_str(&hex::encode(bytes))
    }
}

impl<C> Encode<C> for BoundedBytes {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        // we match the haskell implementation by encoding bytestrings longer than 64
        // bytes as indefinite lists of bytes
        const CHUNK_SIZE: usize = 64;
        let bs: &Vec<u8> = self.deref();
        if bs.len() <= 64 {
            e.bytes(bs)?;
        } else {
            e.begin_bytes()?;
            for b in bs.chunks(CHUNK_SIZE) {
                e.bytes(b)?;
            }
            e.end()?;
        }
        Ok(())
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for BoundedBytes {
    fn decode(d: &mut minicbor::Decoder<'b>, _: &mut C) -> Result<Self, minicbor::decode::Error> {
        let mut res = Vec::new();
        for chunk in d.bytes_iter()? {
            let bs = chunk?;
            res.extend_from_slice(bs);
        }
        Ok(BoundedBytes::from(res))
    }
}

/*
big_int = int / big_uint / big_nint ; New
big_uint = #6.2(bounded_bytes) ; New
big_nint = #6.3(bounded_bytes) ; New
 */

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum BigInt {
    Int(Int),
    BigUInt(BoundedBytes),
    BigNInt(BoundedBytes),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for BigInt {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let datatype = d.datatype()?;

        match datatype {
            minicbor::data::Type::U8
            | minicbor::data::Type::U16
            | minicbor::data::Type::U32
            | minicbor::data::Type::U64
            | minicbor::data::Type::I8
            | minicbor::data::Type::I16
            | minicbor::data::Type::I32
            | minicbor::data::Type::I64
            | minicbor::data::Type::Int => Ok(Self::Int(d.decode_with(ctx)?)),
            minicbor::data::Type::Tag => {
                let tag = d.tag()?;

                match tag {
                    minicbor::data::Tag::PosBignum => Ok(Self::BigUInt(d.decode_with(ctx)?)),
                    minicbor::data::Tag::NegBignum => Ok(Self::BigNInt(d.decode_with(ctx)?)),
                    _ => Err(minicbor::decode::Error::message(
                        "invalid cbor tag for big int",
                    )),
                }
            }
            _ => Err(minicbor::decode::Error::message(
                "invalid cbor data type for big int",
            )),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for BigInt {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            BigInt::Int(x) => {
                e.encode_with(x, ctx)?;
            }
            BigInt::BigUInt(x) => {
                e.tag(Tag::PosBignum)?;
                e.encode_with(x, ctx)?;
            }
            BigInt::BigNInt(x) => {
                e.tag(Tag::NegBignum)?;
                e.encode_with(x, ctx)?;
            }
        };

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum PlutusData {
    Constr(Constr<PlutusData>),
    Map(KeyValuePairs<PlutusData, PlutusData>),
    BigInt(BigInt),
    BoundedBytes(BoundedBytes),
    Array(Vec<PlutusData>),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for PlutusData {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let type_ = d.datatype()?;

        match type_ {
            minicbor::data::Type::Tag => {
                let mut probe = d.probe();
                let tag = probe.tag()?;

                match tag {
                    Tag::Unassigned(121..=127 | 1280..=1400 | 102) => {
                        Ok(Self::Constr(d.decode_with(ctx)?))
                    }
                    Tag::PosBignum | Tag::NegBignum => Ok(Self::BigInt(d.decode_with(ctx)?)),
                    _ => Err(minicbor::decode::Error::message(
                        "unknown tag for plutus data tag",
                    )),
                }
            }
            minicbor::data::Type::U8
            | minicbor::data::Type::U16
            | minicbor::data::Type::U32
            | minicbor::data::Type::U64
            | minicbor::data::Type::I8
            | minicbor::data::Type::I16
            | minicbor::data::Type::I32
            | minicbor::data::Type::I64
            | minicbor::data::Type::Int => Ok(Self::BigInt(d.decode_with(ctx)?)),
            minicbor::data::Type::Map | minicbor::data::Type::MapIndef => {
                Ok(Self::Map(d.decode_with(ctx)?))
            }
            minicbor::data::Type::Bytes => Ok(Self::BoundedBytes(d.decode_with(ctx)?)),
            minicbor::data::Type::BytesIndef => {
                let mut full = Vec::new();

                for slice in d.bytes_iter()? {
                    full.extend(slice?);
                }

                Ok(Self::BoundedBytes(BoundedBytes::from(full)))
            }
            minicbor::data::Type::Array | minicbor::data::Type::ArrayIndef => {
                Ok(Self::Array(d.decode_with(ctx)?))
            }

            _ => Err(minicbor::decode::Error::message(
                "bad cbor data type for plutus data",
            )),
        }
    }
}

fn encode_list<C, W: minicbor::encode::Write, A: minicbor::encode::Encode<C>>(
    a: &Vec<A>,
    e: &mut minicbor::Encoder<W>,
    ctx: &mut C,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    // Mimics default haskell list encoding from cborg:
    // We use indef array for non-empty arrays but definite 0-length array for empty
    if a.is_empty() {
        e.array(0)?;
    } else {
        e.begin_array()?;
        for v in a {
            e.encode_with(v, ctx)?;
        }
        e.end()?;
    }
    Ok(())
}

impl<C> minicbor::encode::Encode<C> for PlutusData {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Self::Constr(a) => {
                e.encode_with(a, ctx)?;
            }
            Self::Map(a) => {
                // we use definite array to match the approach used by haskell's plutus
                // implementation https://github.com/input-output-hk/plutus/blob/9538fc9829426b2ecb0628d352e2d7af96ec8204/plutus-core/plutus-core/src/PlutusCore/Data.hs#L152
                e.map(a.len().try_into().unwrap())?;
                for (k, v) in a.iter() {
                    k.encode(e, ctx)?;
                    v.encode(e, ctx)?;
                }
            }
            Self::BigInt(a) => {
                e.encode_with(a, ctx)?;
            }
            Self::BoundedBytes(a) => {
                e.encode_with(a, ctx)?;
            }
            Self::Array(a) => {
                // we use definite array for empty array or indef array otherwise to match
                // haskell implementation https://github.com/input-output-hk/plutus/blob/9538fc9829426b2ecb0628d352e2d7af96ec8204/plutus-core/plutus-core/src/PlutusCore/Data.hs#L153
                // default encoder for a list:
                // https://github.com/well-typed/cborg/blob/4bdc818a1f0b35f38bc118a87944630043b58384/serialise/src/Codec/Serialise/Class.hs#L181
                encode_list(a, e, ctx)?;
            }
        };

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Constr<A> {
    pub tag: u64,
    pub any_constructor: Option<u64>,
    pub fields: Vec<A>,
}

impl<'b, C, A> minicbor::decode::Decode<'b, C> for Constr<A>
where
    A: minicbor::decode::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let tag = d.tag()?;

        match tag {
            Tag::Unassigned(x) => match x {
                121..=127 | 1280..=1400 => Ok(Constr {
                    tag: x,
                    fields: d.decode_with(ctx)?,
                    any_constructor: None,
                }),
                102 => {
                    d.array()?;

                    Ok(Constr {
                        tag: x,
                        any_constructor: Some(d.decode_with(ctx)?),
                        fields: d.decode_with(ctx)?,
                    })
                }
                _ => Err(minicbor::decode::Error::message(
                    "bad tag code for plutus data",
                )),
            },
            _ => Err(minicbor::decode::Error::message(
                "bad tag code for plutus data",
            )),
        }
    }
}

impl<C, A> minicbor::encode::Encode<C> for Constr<A>
where
    A: minicbor::encode::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.tag(Tag::Unassigned(self.tag))?;

        match self.tag {
            102 => {
                // definite length array here
                // https://github.com/input-output-hk/plutus/blob/9538fc9829426b2ecb0628d352e2d7af96ec8204/plutus-core/plutus-core/src/PlutusCore/Data.hs#L152
                e.array(2)?;
                e.encode_with(self.any_constructor.unwrap_or_default(), ctx)?;

                // we use definite array for empty array or indef array otherwise to match
                // haskell implementation https://github.com/input-output-hk/plutus/blob/9538fc9829426b2ecb0628d352e2d7af96ec8204/plutus-core/plutus-core/src/PlutusCore/Data.hs#L144
                // default encoder for a list:
                // https://github.com/well-typed/cborg/blob/4bdc818a1f0b35f38bc118a87944630043b58384/serialise/src/Codec/Serialise/Class.hs#L181
                encode_list(&self.fields, e, ctx)?;
                Ok(())
            }
            _ => {
                // we use definite array for empty array or indef array otherwise to match
                // haskell implementation. See above reference.
                encode_list(&self.fields, e, ctx)?;
                Ok(())
            }
        }
    }
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct ExUnits {
    #[n(0)]
    pub mem: u32,
    #[n(1)]
    pub steps: u64,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct ExUnitPrices {
    #[n(0)]
    mem_price: PositiveInterval,

    #[n(1)]
    step_price: PositiveInterval,
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

/* bootstrap_witness =
[ public_key : $vkey
, signature  : $signature
, chain_code : bytes .size 32
, attributes : bytes
] */

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct BootstrapWitness {
    #[n(0)]
    pub public_key: Bytes,

    #[n(1)]
    pub signature: Bytes,

    #[n(2)]
    pub chain_code: Bytes,

    #[n(3)]
    pub attributes: Bytes,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct WitnessSet {
    #[n(0)]
    pub vkeywitness: Option<Vec<VKeyWitness>>,

    #[n(1)]
    pub native_script: Option<Vec<NativeScript>>,

    #[n(2)]
    pub bootstrap_witness: Option<Vec<BootstrapWitness>>,

    #[n(3)]
    pub plutus_script: Option<Vec<PlutusScript>>,

    #[n(4)]
    pub plutus_data: Option<Vec<PlutusData>>,

    #[n(5)]
    pub redeemer: Option<Vec<Redeemer>>,
}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
#[cbor(map)]
pub struct MintedWitnessSet<'b> {
    #[n(0)]
    pub vkeywitness: Option<Vec<VKeyWitness>>,

    #[n(1)]
    pub native_script: Option<Vec<NativeScript>>,

    #[n(2)]
    pub bootstrap_witness: Option<Vec<BootstrapWitness>>,

    #[n(3)]
    pub plutus_script: Option<Vec<PlutusScript>>,

    #[b(4)]
    pub plutus_data: Option<Vec<KeepRaw<'b, PlutusData>>>,

    #[n(5)]
    pub redeemer: Option<Vec<Redeemer>>,
}

impl<'b> From<MintedWitnessSet<'b>> for WitnessSet {
    fn from(x: MintedWitnessSet<'b>) -> Self {
        WitnessSet {
            vkeywitness: x.vkeywitness,
            native_script: x.native_script,
            bootstrap_witness: x.bootstrap_witness,
            plutus_script: x.plutus_script,
            plutus_data: x
                .plutus_data
                .map(|x| x.into_iter().map(|x| x.unwrap()).collect()),
            redeemer: x.redeemer,
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
    pub plutus_scripts: Option<Vec<PlutusScript>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Metadatum {
    Int(Int),
    Bytes(Bytes),
    Text(String),
    Array(Vec<Metadatum>),
    Map(KeyValuePairs<Metadatum, Metadatum>),
}

impl<'b, C> minicbor::Decode<'b, C> for Metadatum {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::U8
            | minicbor::data::Type::U16
            | minicbor::data::Type::U32
            | minicbor::data::Type::U64
            | minicbor::data::Type::I8
            | minicbor::data::Type::I16
            | minicbor::data::Type::I32
            | minicbor::data::Type::I64
            | minicbor::data::Type::Int => {
                let i = d.decode()?;
                Ok(Metadatum::Int(i))
            }
            minicbor::data::Type::Bytes => Ok(Metadatum::Bytes(d.decode_with(ctx)?)),
            minicbor::data::Type::String => Ok(Metadatum::Text(d.decode_with(ctx)?)),
            minicbor::data::Type::Array | minicbor::data::Type::ArrayIndef => {
                Ok(Metadatum::Array(d.decode_with(ctx)?))
            }
            minicbor::data::Type::Map | minicbor::data::Type::MapIndef => {
                Ok(Metadatum::Map(d.decode_with(ctx)?))
            }
            _ => Err(minicbor::decode::Error::message(
                "Can't turn data type into metadatum",
            )),
        }
    }
}

impl<C> minicbor::Encode<C> for Metadatum {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Metadatum::Int(a) => {
                e.encode_with(a, ctx)?;
            }
            Metadatum::Bytes(a) => {
                e.encode_with(a, ctx)?;
            }
            Metadatum::Text(a) => {
                e.encode_with(a, ctx)?;
            }
            Metadatum::Array(a) => {
                e.encode_with(a, ctx)?;
            }
            Metadatum::Map(a) => {
                e.encode_with(a, ctx)?;
            }
        };

        Ok(())
    }
}

pub type MetadatumLabel = u64;

pub type Metadata = KeyValuePairs<MetadatumLabel, Metadatum>;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
pub struct ShelleyMaAuxiliaryData {
    #[n(0)]
    pub transaction_metadata: Metadata,

    #[n(1)]
    pub auxiliary_scripts: Option<Vec<NativeScript>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum AuxiliaryData {
    Shelley(Metadata),
    ShelleyMa(ShelleyMaAuxiliaryData),
    PostAlonzo(PostAlonzoAuxiliaryData),
}

impl<'b, C> minicbor::Decode<'b, C> for AuxiliaryData {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Map | minicbor::data::Type::MapIndef => {
                Ok(AuxiliaryData::Shelley(d.decode_with(ctx)?))
            }
            minicbor::data::Type::Array => Ok(AuxiliaryData::ShelleyMa(d.decode_with(ctx)?)),
            minicbor::data::Type::Tag => {
                d.tag()?;
                Ok(AuxiliaryData::PostAlonzo(d.decode_with(ctx)?))
            }
            _ => Err(minicbor::decode::Error::message(
                "Can't infer variant from data type for AuxiliaryData",
            )),
        }
    }
}

impl<C> minicbor::Encode<C> for AuxiliaryData {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            AuxiliaryData::Shelley(m) => {
                e.encode_with(m, ctx)?;
            }
            AuxiliaryData::ShelleyMa(m) => {
                e.encode_with(m, ctx)?;
            }
            AuxiliaryData::PostAlonzo(v) => {
                // TODO: check if this is the correct tag
                e.tag(Tag::Unassigned(259))?;
                e.encode_with(v, ctx)?;
            }
        };

        Ok(())
    }
}

pub type TransactionIndex = u32;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Clone)]
pub struct Block {
    #[n(0)]
    pub header: Header,

    #[b(1)]
    pub transaction_bodies: Vec<TransactionBody>,

    #[n(2)]
    pub transaction_witness_sets: Vec<WitnessSet>,

    #[n(3)]
    pub auxiliary_data_set: KeyValuePairs<TransactionIndex, AuxiliaryData>,

    #[n(4)]
    pub invalid_transactions: Option<Vec<TransactionIndex>>,
}

/// A memory representation of an already minted block
///
/// This structure is analogous to [Block], but it allows to retrieve the
/// original CBOR bytes for each structure that might require hashing. In this
/// way, we make sure that the resulting hash matches what exists on-chain.
#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct MintedBlock<'b> {
    #[n(0)]
    pub header: KeepRaw<'b, Header>,

    #[b(1)]
    pub transaction_bodies: MaybeIndefArray<KeepRaw<'b, TransactionBody>>,

    #[n(2)]
    pub transaction_witness_sets: MaybeIndefArray<KeepRaw<'b, MintedWitnessSet<'b>>>,

    #[n(3)]
    pub auxiliary_data_set: KeyValuePairs<TransactionIndex, KeepRaw<'b, AuxiliaryData>>,

    #[n(4)]
    pub invalid_transactions: Option<MaybeIndefArray<TransactionIndex>>,
}

impl<'b> From<MintedBlock<'b>> for Block {
    fn from(x: MintedBlock<'b>) -> Self {
        Block {
            header: x.header.unwrap(),
            transaction_bodies: x
                .transaction_bodies
                .to_vec()
                .into_iter()
                .map(|x| x.unwrap())
                .collect(),
            transaction_witness_sets: x
                .transaction_witness_sets
                .to_vec()
                .into_iter()
                .map(|x| x.unwrap())
                .map(WitnessSet::from)
                .collect(),
            auxiliary_data_set: x
                .auxiliary_data_set
                .to_vec()
                .into_iter()
                .map(|(k, v)| (k, v.unwrap()))
                .collect::<Vec<_>>()
                .into(),
            invalid_transactions: x.invalid_transactions.map(|x| x.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug)]
pub struct Tx {
    #[n(0)]
    pub transaction_body: TransactionBody,

    #[n(1)]
    pub transaction_witness_set: WitnessSet,

    #[n(2)]
    pub success: bool,

    #[n(3)]
    pub auxiliary_data: Nullable<AuxiliaryData>,
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct MintedTx<'b> {
    #[b(0)]
    pub transaction_body: KeepRaw<'b, TransactionBody>,

    #[n(1)]
    pub transaction_witness_set: KeepRaw<'b, MintedWitnessSet<'b>>,

    #[n(2)]
    pub success: bool,

    #[n(3)]
    pub auxiliary_data: Nullable<KeepRaw<'b, AuxiliaryData>>,
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor::{self, to_vec};

    use crate::{alonzo::PlutusData, Fragment};

    use super::{Header, MintedBlock};

    type BlockWrapper<'b> = (u16, MintedBlock<'b>);

    #[test]
    fn block_isomorphic_decoding_encoding() {
        let test_blocks = vec![
            include_str!("../../../test_data/alonzo1.block"),
            include_str!("../../../test_data/alonzo2.block"),
            include_str!("../../../test_data/alonzo3.block"),
            include_str!("../../../test_data/alonzo4.block"),
            include_str!("../../../test_data/alonzo5.block"),
            include_str!("../../../test_data/alonzo6.block"),
            include_str!("../../../test_data/alonzo7.block"),
            include_str!("../../../test_data/alonzo8.block"),
            include_str!("../../../test_data/alonzo9.block"),
            // old block without invalid_transactions fields
            include_str!("../../../test_data/alonzo10.block"),
            // peculiar block with protocol update params
            include_str!("../../../test_data/alonzo11.block"),
            // peculiar block with decoding issue
            // https://github.com/txpipe/oura/issues/37
            include_str!("../../../test_data/alonzo12.block"),
            // peculiar block with protocol update params, including nonce
            include_str!("../../../test_data/alonzo13.block"),
            // peculiar block with overflow crash
            // https://github.com/txpipe/oura/issues/113
            include_str!("../../../test_data/alonzo14.block"),
            // peculiar block with many move-instantaneous-rewards certs
            include_str!("../../../test_data/alonzo15.block"),
            // peculiar block with protocol update values
            include_str!("../../../test_data/alonzo16.block"),
            // peculiar block with missing nonce hash
            include_str!("../../../test_data/alonzo17.block"),
            // peculiar block with strange AuxiliaryData variant
            include_str!("../../../test_data/alonzo18.block"),
            // peculiar block with strange AuxiliaryData variant
            include_str!("../../../test_data/alonzo18.block"),
            // peculiar block with nevative i64 overflow
            include_str!("../../../test_data/alonzo19.block"),
            // peculiar block with very BigInt in plutus code
            include_str!("../../../test_data/alonzo20.block"),
            // peculiar block with bad tx hash
            include_str!("../../../test_data/alonzo21.block"),
            // peculiar block with bad tx hash
            include_str!("../../../test_data/alonzo22.block"),
            // peculiar block with indef byte array in plutus data
            include_str!("../../../test_data/alonzo23.block"),
            // peculiar block with invalid address (pointer overflow)
            include_str!("../../../test_data/alonzo27.block"),
        ];

        for (idx, block_str) in test_blocks.iter().enumerate() {
            println!("decoding test block {}", idx + 1);
            let bytes = hex::decode(block_str).unwrap_or_else(|_| panic!("bad block file {idx}"));

            let block: BlockWrapper = minicbor::decode(&bytes[..])
                .unwrap_or_else(|_| panic!("error decoding cbor for file {idx}"));

            let bytes2 = to_vec(block)
                .unwrap_or_else(|_| panic!("error encoding block cbor for file {idx}"));

            assert!(bytes.eq(&bytes2), "re-encoded bytes didn't match original");
        }
    }

    #[test]
    fn header_isomorphic_decoding_encoding() {
        let test_headers = [
            // peculiar alonzo header used as origin for a vasil devnet
            include_str!("../../../test_data/alonzo26.header"),
        ];

        for (idx, header_str) in test_headers.iter().enumerate() {
            println!("decoding test header {}", idx + 1);
            let bytes = hex::decode(header_str).unwrap_or_else(|_| panic!("bad header file {idx}"));

            let header: Header = minicbor::decode(&bytes[..])
                .unwrap_or_else(|_| panic!("error decoding cbor for file {idx}"));

            let bytes2 = to_vec(header)
                .unwrap_or_else(|_| panic!("error encoding header cbor for file {idx}"));

            assert!(bytes.eq(&bytes2), "re-encoded bytes didn't match original");
        }
    }

    #[test]
    fn plutus_data_isomorphic_decoding_encoding() {
        let datas = [
            // unit = Constr 0 []
            "d87980",
            // pltmap = Map [(I 1, unit), (I 2, pltlist)]
            "a201d87980029f000102ff",
            // pltlist = List [I 0, I 1, I 2]
            "9f000102ff",
            // Constr 5 [pltmap, Constr 5 [Map [(pltmap, toData True), (pltlist, pltmap), (List [], List [I 1])], unit, toData (0, 1)]]
            "d87e9fa201d87980029f000102ffd87e9fa3a201d87980029f000102ffd87a809f000102ffa201d87980029f000102ff809f01ffd87980d8799f0001ffffff",
            // Constr 5 [List [], List [I 1], Map [], Map [(I 1, unit), (I 2, Constr 2 [I 2])]]
            "d87e9f809f01ffa0a201d8798002d87b9f02ffff",
            // B (B.replicate 32 105)
            "58206969696969696969696969696969696969696969696969696969696969696969",
            // B (B.replicate 67 105)
            "5f58406969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696969696943696969ff",
            // B B.empty
            "40"
        ];
        for data_hex in datas {
            let data_bytes = hex::decode(data_hex).unwrap();
            let data = PlutusData::decode_fragment(&data_bytes).unwrap();
            assert_eq!(data.encode_fragment().unwrap(), data_bytes);
        }
    }
}
