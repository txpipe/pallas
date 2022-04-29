//! Ledger primitives and cbor codec for the Alonzo era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Alonzo CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/alonzo/test-suite/cddl-files/alonzo.cddl) file in IOHK repo.

use pallas_codec::minicbor::{bytes::ByteVec, data::Int, data::Tag, Decode, Encode};
use pallas_crypto::hash::Hash;
use std::ops::Deref;

use pallas_codec::utils::{AnyUInt, KeyValuePairs, MaybeIndefArray};

// required for derive attrs to work
use pallas_codec::minicbor;

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct VrfCert(#[n(0)] pub ByteVec, #[n(1)] pub ByteVec);

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct HeaderBody {
    #[n(0)]
    pub block_number: u64,

    #[n(1)]
    pub slot: u64,

    #[n(2)]
    pub prev_hash: Hash<32>,

    #[n(3)]
    pub issuer_vkey: ByteVec,

    #[n(4)]
    pub vrf_vkey: ByteVec,

    #[n(5)]
    pub nonce_vrf: VrfCert,

    #[n(6)]
    pub leader_vrf: VrfCert,

    #[n(7)]
    pub block_body_size: u64,

    #[n(8)]
    pub block_body_hash: Hash<32>,

    #[n(9)]
    pub operational_cert: ByteVec,

    #[n(10)]
    pub unknown_0: u64,

    #[n(11)]
    pub unknown_1: u64,

    #[n(12)]
    pub unknown_2: ByteVec,

    #[n(13)]
    pub protocol_version_major: u64,

    #[n(14)]
    pub protocol_version_minor: u64,
}

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct KesSignature {}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct Header {
    #[n(0)]
    pub header_body: HeaderBody,

    #[n(1)]
    pub body_signature: ByteVec,
}

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct TransactionInput {
    #[n(0)]
    pub transaction_id: Hash<32>,

    #[n(1)]
    pub index: u64,
}

// $nonce /= [ 0 // 1, bytes .size 32 ]

#[derive(Encode, Decode, Debug, PartialEq)]
#[cbor(index_only)]
pub enum NonceVariant {
    #[n(0)]
    NeutralNonce,

    #[n(1)]
    Nonce,
}

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct Nonce {
    #[n(0)]
    pub variant: NonceVariant,

    #[n(1)]
    pub hash: Option<Hash<32>>,
}

pub type ScriptHash = ByteVec;

pub type PolicyId = ScriptHash;

pub type AssetName = ByteVec;

pub type Multiasset<A> = KeyValuePairs<PolicyId, KeyValuePairs<AssetName, A>>;

pub type Mint = Multiasset<i64>;

pub type Coin = AnyUInt;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Coin(Coin),
    Multiasset(Coin, Multiasset<Coin>),
}

impl<'b> minicbor::decode::Decode<'b> for Value {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::U32 => Ok(Value::Coin(d.decode()?)),
            minicbor::data::Type::U64 => Ok(Value::Coin(d.decode()?)),
            minicbor::data::Type::Array => {
                d.array()?;
                let coin = d.decode()?;
                let multiasset = d.decode()?;
                Ok(Value::Multiasset(coin, multiasset))
            }
            _ => Err(minicbor::decode::Error::message(
                "unknown cbor data type for Alonzo Value enum",
            )),
        }
    }
}

impl minicbor::encode::Encode for Value {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        // TODO: check how to deal with uint variants (u32 vs u64)
        match self {
            Value::Coin(coin) => {
                e.encode(coin)?;
            }
            Value::Multiasset(coin, other) => {
                e.array(2)?;
                e.encode(coin)?;
                e.encode(other)?;
            }
        };

        Ok(())
    }
}

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct TransactionOutput {
    #[n(0)]
    pub address: ByteVec,

    #[n(1)]
    pub amount: Value,

    #[n(2)]
    pub datum_hash: Option<Hash<32>>,
}

pub type PoolKeyhash = Hash<28>;
pub type Epoch = u64;
pub type Genesishash = ByteVec;
pub type GenesisDelegateHash = ByteVec;
pub type VrfKeyhash = Hash<32>;

/* move_instantaneous_reward = [ 0 / 1, { * stake_credential => delta_coin } / coin ]
; The first field determines where the funds are drawn from.
; 0 denotes the reserves, 1 denotes the treasury.
; If the second field is a map, funds are moved to stake credentials,
; otherwise the funds are given to the other accounting pot.
 */

#[derive(Debug, PartialEq, PartialOrd)]
pub enum InstantaneousRewardSource {
    Reserves,
    Treasury,
}

impl<'b> minicbor::decode::Decode<'b> for InstantaneousRewardSource {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let variant = d.u32()?;

        match variant {
            0 => Ok(Self::Reserves),
            1 => Ok(Self::Treasury),
            _ => Err(minicbor::decode::Error::message("invalid funds variant")),
        }
    }
}

impl minicbor::encode::Encode for InstantaneousRewardSource {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let variant = match self {
            Self::Reserves => 0,
            Self::Treasury => 1,
        };

        e.u32(variant)?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum InstantaneousRewardTarget {
    StakeCredentials(KeyValuePairs<StakeCredential, i64>),
    OtherAccountingPot(Coin),
}

impl<'b> minicbor::decode::Decode<'b> for InstantaneousRewardTarget {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let datatype = d.datatype()?;

        match datatype {
            minicbor::data::Type::Map | minicbor::data::Type::MapIndef => {
                let a = d.decode()?;
                Ok(Self::StakeCredentials(a))
            }
            _ => {
                let a = d.decode()?;
                Ok(Self::OtherAccountingPot(a))
            }
        }
    }
}

impl minicbor::encode::Encode for InstantaneousRewardTarget {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            InstantaneousRewardTarget::StakeCredentials(a) => {
                a.encode(e)?;
                Ok(())
            }
            InstantaneousRewardTarget::OtherAccountingPot(a) => {
                a.encode(e)?;
                Ok(())
            }
        }
    }
}

#[derive(Encode, Decode, Debug, PartialEq, PartialOrd)]
#[cbor]
pub struct MoveInstantaneousReward {
    #[n(0)]
    pub source: InstantaneousRewardSource,

    #[n(1)]
    pub target: InstantaneousRewardTarget,
}

pub type RewardAccount = ByteVec;

pub type Port = u32;
pub type IPv4 = ByteVec;
pub type IPv6 = ByteVec;
pub type DnsName = String;

#[derive(Debug, PartialEq)]
pub enum Relay {
    SingleHostAddr(Option<Port>, Option<IPv4>, Option<IPv6>),
    SingleHostName(Option<Port>, DnsName),
    MultiHostName(DnsName),
}

impl<'b> minicbor::decode::Decode<'b> for Relay {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => Ok(Relay::SingleHostAddr(d.decode()?, d.decode()?, d.decode()?)),
            1 => Ok(Relay::SingleHostName(d.decode()?, d.decode()?)),
            2 => Ok(Relay::MultiHostName(d.decode()?)),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant id for Relay",
            )),
        }
    }
}

impl minicbor::encode::Encode for Relay {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Relay::SingleHostAddr(a, b, c) => {
                e.array(4)?;
                e.encode(0)?;
                e.encode(a)?;
                e.encode(b)?;
                e.encode(c)?;

                Ok(())
            }
            Relay::SingleHostName(a, b) => {
                e.array(3)?;
                e.encode(1)?;
                e.encode(a)?;
                e.encode(b)?;

                Ok(())
            }
            Relay::MultiHostName(a) => {
                e.array(2)?;
                e.encode(2)?;
                e.encode(a)?;

                Ok(())
            }
        }
    }
}

pub type PoolMetadataHash = Hash<32>;

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct PoolMetadata {
    #[n(0)]
    pub url: String,

    #[n(1)]
    pub hash: PoolMetadataHash,
}

pub type AddrKeyhash = Hash<28>;
pub type Scripthash = Hash<28>;

#[derive(Debug, PartialEq)]
pub struct RationalNumber {
    pub numerator: i64,
    pub denominator: u64,
}

impl<'b> minicbor::decode::Decode<'b> for RationalNumber {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.tag()?;
        d.array()?;

        Ok(RationalNumber {
            numerator: d.decode()?,
            denominator: d.decode()?,
        })
    }
}

impl minicbor::encode::Encode for RationalNumber {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        // TODO: check if this is the correct tag
        e.tag(Tag::Unassigned(30))?;
        e.array(2)?;
        e.encode(self.numerator)?;
        e.encode(self.denominator)?;

        Ok(())
    }
}

pub type UnitInterval = RationalNumber;

pub type PositiveInterval = RationalNumber;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum StakeCredential {
    AddrKeyhash(AddrKeyhash),
    Scripthash(Scripthash),
}

impl<'b> minicbor::decode::Decode<'b> for StakeCredential {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => Ok(StakeCredential::AddrKeyhash(d.decode()?)),
            1 => Ok(StakeCredential::Scripthash(d.decode()?)),
            _ => Err(minicbor::decode::Error::message(
                "invalid variant id for StakeCredential",
            )),
        }
    }
}

impl minicbor::encode::Encode for StakeCredential {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            StakeCredential::AddrKeyhash(a) => {
                e.array(2)?;
                e.encode(0)?;
                e.encode(a)?;

                Ok(())
            }
            StakeCredential::Scripthash(a) => {
                e.array(2)?;
                e.encode(1)?;
                e.encode(a)?;

                Ok(())
            }
        }
    }
}

#[derive(Debug, PartialEq)]
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
        pool_owners: MaybeIndefArray<AddrKeyhash>,
        relays: MaybeIndefArray<Relay>,
        pool_metadata: Option<PoolMetadata>,
    },
    PoolRetirement(PoolKeyhash, Epoch),
    GenesisKeyDelegation(Genesishash, GenesisDelegateHash, VrfKeyhash),
    MoveInstantaneousRewardsCert(MoveInstantaneousReward),
}

impl<'b> minicbor::decode::Decode<'b> for Certificate {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u16()?;

        match variant {
            0 => {
                let a = d.decode()?;
                Ok(Certificate::StakeRegistration(a))
            }
            1 => {
                let a = d.decode()?;
                Ok(Certificate::StakeDeregistration(a))
            }
            2 => {
                let a = d.decode()?;
                let b = d.decode()?;
                Ok(Certificate::StakeDelegation(a, b))
            }
            3 => {
                let operator = d.decode()?;
                let vrf_keyhash = d.decode()?;
                let pledge = d.decode()?;
                let cost = d.decode()?;
                let margin = d.decode()?;
                let reward_account = d.decode()?;
                let pool_owners = d.decode()?;
                let relays = d.decode()?;
                let pool_metadata = d.decode()?;

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
                let a = d.decode()?;
                let b = d.decode()?;
                Ok(Certificate::PoolRetirement(a, b))
            }
            5 => {
                let a = d.decode()?;
                let b = d.decode()?;
                let c = d.decode()?;
                Ok(Certificate::GenesisKeyDelegation(a, b, c))
            }
            6 => {
                let a = d.decode()?;
                Ok(Certificate::MoveInstantaneousRewardsCert(a))
            }
            _ => Err(minicbor::decode::Error::message(
                "unknown variant id for certificate",
            )),
        }
    }
}

impl minicbor::encode::Encode for Certificate {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Certificate::StakeRegistration(a) => {
                e.array(2)?;
                e.u16(0)?;
                e.encode(a)?;

                Ok(())
            }
            Certificate::StakeDeregistration(a) => {
                e.array(2)?;
                e.u16(1)?;
                e.encode(a)?;

                Ok(())
            }
            Certificate::StakeDelegation(a, b) => {
                e.array(3)?;
                e.u16(2)?;
                e.encode(a)?;
                e.encode(b)?;

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

                e.encode(operator)?;
                e.encode(vrf_keyhash)?;
                e.encode(pledge)?;
                e.encode(cost)?;
                e.encode(margin)?;
                e.encode(reward_account)?;
                e.encode(pool_owners)?;
                e.encode(relays)?;
                e.encode(pool_metadata)?;

                Ok(())
            }
            Certificate::PoolRetirement(a, b) => {
                e.array(3)?;
                e.u16(4)?;
                e.encode(a)?;
                e.encode(b)?;

                Ok(())
            }
            Certificate::GenesisKeyDelegation(a, b, c) => {
                e.array(4)?;
                e.u16(5)?;
                e.encode(a)?;
                e.encode(b)?;
                e.encode(c)?;

                Ok(())
            }
            Certificate::MoveInstantaneousRewardsCert(a) => {
                e.array(2)?;
                e.u16(6)?;
                e.encode(a)?;

                Ok(())
            }
        }
    }
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cbor(index_only)]
pub enum NetworkId {
    #[n(0)]
    One,
    #[n(1)]
    Two,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[cbor(index_only)]
pub enum Language {
    #[n(0)]
    PlutusV1,
}

pub type CostModel = MaybeIndefArray<i32>;

pub type CostMdls = KeyValuePairs<Language, CostModel>;

pub type ProtocolVersion = (u32, u32);

#[derive(Encode, Decode, Debug, PartialEq)]
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

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct Update {
    #[n(0)]
    pub proposed_protocol_parameter_updates: KeyValuePairs<Genesishash, ProtocolParamUpdate>,

    #[n(1)]
    pub epoch: Epoch,
}

#[derive(Debug, PartialEq)]
pub enum TransactionBodyComponent {
    Inputs(MaybeIndefArray<TransactionInput>),
    Outputs(MaybeIndefArray<TransactionOutput>),
    Fee(u64),
    Ttl(u64),
    Certificates(MaybeIndefArray<Certificate>),
    Withdrawals(KeyValuePairs<RewardAccount, Coin>),
    Update(Update),
    AuxiliaryDataHash(ByteVec),
    ValidityIntervalStart(u64),
    Mint(Multiasset<i64>),
    ScriptDataHash(Hash<32>),
    Collateral(MaybeIndefArray<TransactionInput>),
    RequiredSigners(MaybeIndefArray<AddrKeyhash>),
    NetworkId(NetworkId),
}

impl<'b> minicbor::decode::Decode<'b> for TransactionBodyComponent {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let key: u32 = d.decode()?;

        match key {
            0 => Ok(Self::Inputs(d.decode()?)),
            1 => Ok(Self::Outputs(d.decode()?)),
            2 => Ok(Self::Fee(d.decode()?)),
            3 => Ok(Self::Ttl(d.decode()?)),
            4 => Ok(Self::Certificates(d.decode()?)),
            5 => Ok(Self::Withdrawals(d.decode()?)),
            6 => Ok(Self::Update(d.decode()?)),
            7 => Ok(Self::AuxiliaryDataHash(d.decode()?)),
            8 => Ok(Self::ValidityIntervalStart(d.decode()?)),
            9 => Ok(Self::Mint(d.decode()?)),
            11 => Ok(Self::ScriptDataHash(d.decode()?)),
            13 => Ok(Self::Collateral(d.decode()?)),
            14 => Ok(Self::RequiredSigners(d.decode()?)),
            15 => Ok(Self::NetworkId(d.decode()?)),
            _ => Err(minicbor::decode::Error::message(
                "invalid map key for transaction body component",
            )),
        }
    }
}

impl minicbor::encode::Encode for TransactionBodyComponent {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            TransactionBodyComponent::Inputs(x) => {
                e.encode(0)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::Outputs(x) => {
                e.encode(1)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::Fee(x) => {
                e.encode(2)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::Ttl(x) => {
                e.encode(3)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::Certificates(x) => {
                e.encode(4)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::Withdrawals(x) => {
                e.encode(5)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::Update(x) => {
                e.encode(6)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::AuxiliaryDataHash(x) => {
                e.encode(7)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::ValidityIntervalStart(x) => {
                e.encode(8)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::Mint(x) => {
                e.encode(9)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::ScriptDataHash(x) => {
                e.encode(11)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::Collateral(x) => {
                e.encode(13)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::RequiredSigners(x) => {
                e.encode(14)?;
                e.encode(x)?;
            }
            TransactionBodyComponent::NetworkId(x) => {
                e.encode(15)?;
                e.encode(x)?;
            }
        }

        Ok(())
    }
}

// Can't derive encode for TransactionBody because it seems to require a very
// particular order for each key in the map
#[derive(Debug, PartialEq)]
pub struct TransactionBody(Vec<TransactionBodyComponent>);

impl Deref for TransactionBody {
    type Target = Vec<TransactionBodyComponent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'b> minicbor::decode::Decode<'b> for TransactionBody {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let len = d.map()?.unwrap_or_default();

        let components: Result<_, _> = (0..len).map(|_| d.decode()).collect();

        Ok(Self(components?))
    }
}

impl minicbor::encode::Encode for TransactionBody {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.map(self.0.len() as u64)?;
        for component in &self.0 {
            e.encode(component)?;
        }

        Ok(())
    }
}

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct VKeyWitness {
    #[n(0)]
    pub vkey: ByteVec,

    #[n(1)]
    pub signature: ByteVec,
}

#[derive(Debug, PartialEq)]
pub enum NativeScript {
    ScriptPubkey(AddrKeyhash),
    ScriptAll(MaybeIndefArray<NativeScript>),
    ScriptAny(MaybeIndefArray<NativeScript>),
    ScriptNOfK(u32, MaybeIndefArray<NativeScript>),
    InvalidBefore(u64),
    InvalidHereafter(u64),
}

impl<'b> minicbor::decode::Decode<'b> for NativeScript {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let variant = d.u32()?;

        match variant {
            0 => Ok(NativeScript::ScriptPubkey(d.decode()?)),
            1 => Ok(NativeScript::ScriptAll(d.decode()?)),
            2 => Ok(NativeScript::ScriptAny(d.decode()?)),
            3 => Ok(NativeScript::ScriptNOfK(d.decode()?, d.decode()?)),
            4 => Ok(NativeScript::InvalidBefore(d.decode()?)),
            5 => Ok(NativeScript::InvalidHereafter(d.decode()?)),
            _ => Err(minicbor::decode::Error::message(
                "unknown variant id for native script",
            )),
        }
    }
}

impl minicbor::encode::Encode for NativeScript {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(2)?;

        match self {
            NativeScript::ScriptPubkey(v) => {
                e.encode(0)?;
                e.encode(v)?;
            }
            NativeScript::ScriptAll(v) => {
                e.encode(1)?;
                e.encode(v)?;
            }
            NativeScript::ScriptAny(v) => {
                e.encode(2)?;
                e.encode(v)?;
            }
            NativeScript::ScriptNOfK(a, b) => {
                e.encode(3)?;
                e.encode(a)?;
                e.encode(b)?;
            }
            NativeScript::InvalidBefore(v) => {
                e.encode(4)?;
                e.encode(v)?;
            }
            NativeScript::InvalidHereafter(v) => {
                e.encode(5)?;
                e.encode(v)?;
            }
        }

        Ok(())
    }
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[cbor(transparent)]
pub struct PlutusScript(#[n(0)] ByteVec);

/*
big_int = int / big_uint / big_nint ; New
big_uint = #6.2(bounded_bytes) ; New
big_nint = #6.3(bounded_bytes) ; New
 */

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BigInt {
    Int(Int),
    BigUInt(ByteVec),
    BigNInt(ByteVec),
}

impl<'b> minicbor::decode::Decode<'b> for BigInt {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let datatype = d.datatype()?;

        match datatype {
            minicbor::data::Type::U8
            | minicbor::data::Type::U16
            | minicbor::data::Type::U32
            | minicbor::data::Type::U64
            | minicbor::data::Type::I8
            | minicbor::data::Type::I16
            | minicbor::data::Type::I32
            | minicbor::data::Type::I64 => Ok(Self::Int(d.decode()?)),
            minicbor::data::Type::Tag => {
                let tag = d.tag()?;

                match tag {
                    minicbor::data::Tag::PosBignum => Ok(Self::BigUInt(d.decode()?)),
                    minicbor::data::Tag::NegBignum => Ok(Self::BigNInt(d.decode()?)),
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

impl minicbor::encode::Encode for BigInt {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            BigInt::Int(x) => {
                e.encode(x)?;
            }
            BigInt::BigUInt(x) => {
                e.tag(Tag::PosBignum)?;
                e.encode(x)?;
            }
            BigInt::BigNInt(x) => {
                e.tag(Tag::NegBignum)?;
                e.encode(x)?;
            }
        };

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlutusData {
    Constr(Constr<PlutusData>),
    Map(KeyValuePairs<PlutusData, PlutusData>),
    BigInt(BigInt),
    BoundedBytes(ByteVec),
    Array(MaybeIndefArray<PlutusData>),
    ArrayIndef(MaybeIndefArray<PlutusData>),
}

impl<'b> minicbor::decode::Decode<'b> for PlutusData {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let type_ = d.datatype()?;

        match type_ {
            minicbor::data::Type::Tag => {
                let mut probe = d.probe();
                let tag = probe.tag()?;

                match tag {
                    Tag::Unassigned(121..=127 | 1280..=1400 | 102) => Ok(Self::Constr(d.decode()?)),
                    Tag::PosBignum | Tag::NegBignum => Ok(Self::BigInt(d.decode()?)),
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
            | minicbor::data::Type::I64 => Ok(Self::BigInt(d.decode()?)),
            minicbor::data::Type::Map => Ok(Self::Map(d.decode()?)),
            minicbor::data::Type::Bytes => Ok(Self::BoundedBytes(d.decode()?)),
            minicbor::data::Type::BytesIndef => Ok(Self::BoundedBytes(d.decode()?)),
            minicbor::data::Type::Array => Ok(Self::Array(d.decode()?)),
            minicbor::data::Type::ArrayIndef => Ok(Self::ArrayIndef(d.decode()?)),

            _ => Err(minicbor::decode::Error::message(
                "bad cbor data type for plutus data",
            )),
        }
    }
}

impl minicbor::encode::Encode for PlutusData {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Self::Constr(a) => {
                e.encode(a)?;
            }
            Self::Map(a) => {
                e.encode(a)?;
            }
            Self::BigInt(a) => {
                e.encode(a)?;
            }
            Self::BoundedBytes(a) => {
                e.encode(a)?;
            }
            Self::Array(a) => {
                e.encode(a)?;
            }
            Self::ArrayIndef(a) => {
                e.encode(a)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Constr<A> {
    pub tag: u64,
    pub any_constructor: Option<u64>,
    pub fields: MaybeIndefArray<A>,
}

impl<'b, A> minicbor::decode::Decode<'b> for Constr<A>
where
    A: minicbor::decode::Decode<'b>,
{
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let tag = d.tag()?;

        match tag {
            Tag::Unassigned(x) => match x {
                121..=127 | 1280..=1400 => Ok(Constr {
                    tag: x,
                    fields: d.decode()?,
                    any_constructor: None,
                }),
                102 => {
                    d.array()?;

                    Ok(Constr {
                        tag: x,
                        any_constructor: Some(d.decode()?),
                        fields: d.decode()?,
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

impl<A> minicbor::encode::Encode for Constr<A>
where
    A: minicbor::encode::Encode,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.tag(Tag::Unassigned(self.tag))?;

        match self.tag {
            102 => {
                e.array(2)?;
                e.encode(self.any_constructor.unwrap_or_default())?;
                e.encode(&self.fields)?;

                Ok(())
            }
            _ => {
                e.encode(&self.fields)?;

                Ok(())
            }
        }
    }
}

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct ExUnits {
    #[n(0)]
    pub mem: u32,
    #[n(1)]
    pub steps: u64,
}

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct ExUnitPrices {
    #[n(0)]
    mem_price: PositiveInterval,

    #[n(1)]
    step_price: PositiveInterval,
}

#[derive(Encode, Decode, Debug, PartialEq)]
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

#[derive(Encode, Decode, Debug, PartialEq)]
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

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct BootstrapWitness {
    #[n(0)]
    pub public_key: ByteVec,

    #[n(1)]
    pub signature: ByteVec,

    #[n(2)]
    pub chain_code: ByteVec,

    #[n(3)]
    pub attributes: ByteVec,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[cbor(map)]
pub struct TransactionWitnessSet {
    #[n(0)]
    pub vkeywitness: Option<MaybeIndefArray<VKeyWitness>>,

    #[n(1)]
    pub native_script: Option<MaybeIndefArray<NativeScript>>,

    #[n(2)]
    pub bootstrap_witness: Option<MaybeIndefArray<BootstrapWitness>>,

    #[n(3)]
    pub plutus_script: Option<MaybeIndefArray<PlutusScript>>,

    #[n(4)]
    pub plutus_data: Option<MaybeIndefArray<PlutusData>>,

    #[n(5)]
    pub redeemer: Option<MaybeIndefArray<Redeemer>>,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[cbor(map)]
pub struct AlonzoAuxiliaryData {
    #[n(0)]
    pub metadata: Option<Metadata>,
    #[n(1)]
    pub native_scripts: Option<MaybeIndefArray<NativeScript>>,
    #[n(2)]
    pub plutus_scripts: Option<MaybeIndefArray<PlutusScript>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Metadatum {
    Int(Int),
    Bytes(ByteVec),
    Text(String),
    Array(MaybeIndefArray<Metadatum>),
    Map(KeyValuePairs<Metadatum, Metadatum>),
}

impl<'b> minicbor::Decode<'b> for Metadatum {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::U8 => {
                let i = d.u8()?;
                Ok(Metadatum::Int(i.into()))
            }
            minicbor::data::Type::U16 => {
                let i = d.u16()?;
                Ok(Metadatum::Int(i.into()))
            }
            minicbor::data::Type::U32 => {
                let i = d.u32()?;
                Ok(Metadatum::Int(i.into()))
            }
            minicbor::data::Type::U64 => {
                let i = d.u64()?;
                Ok(Metadatum::Int(i.into()))
            }
            minicbor::data::Type::I8 => {
                let i = d.i8()?;
                Ok(Metadatum::Int(i.into()))
            }
            minicbor::data::Type::I16 => {
                let i = d.i16()?;
                Ok(Metadatum::Int(i.into()))
            }
            minicbor::data::Type::I32 => {
                let i = d.i32()?;
                Ok(Metadatum::Int(i.into()))
            }
            minicbor::data::Type::I64 => {
                let i = d.i64()?;
                Ok(Metadatum::Int(i.into()))
            }
            minicbor::data::Type::Int => {
                let i = d.int()?;
                Ok(Metadatum::Int(i))
            }
            minicbor::data::Type::Bytes => Ok(Metadatum::Bytes(d.decode()?)),
            minicbor::data::Type::String => Ok(Metadatum::Text(d.decode()?)),
            minicbor::data::Type::Array => Ok(Metadatum::Array(d.decode()?)),
            minicbor::data::Type::Map => Ok(Metadatum::Map(d.decode()?)),
            _ => Err(minicbor::decode::Error::message(
                "Can't turn data type into metadatum",
            )),
        }
    }
}

impl minicbor::Encode for Metadatum {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Metadatum::Int(a) => {
                e.encode(a)?;
            }
            Metadatum::Bytes(a) => {
                e.encode(a)?;
            }
            Metadatum::Text(a) => {
                e.encode(a)?;
            }
            Metadatum::Array(a) => {
                e.encode(a)?;
            }
            Metadatum::Map(a) => {
                e.encode(a)?;
            }
        };

        Ok(())
    }
}

pub type MetadatumLabel = AnyUInt;

pub type Metadata = KeyValuePairs<MetadatumLabel, Metadatum>;

#[derive(Debug, PartialEq)]
pub enum AuxiliaryData {
    Shelley(Metadata),
    ShelleyMa {
        transaction_metadata: Metadata,
        auxiliary_scripts: Option<MaybeIndefArray<NativeScript>>,
    },
    Alonzo(AlonzoAuxiliaryData),
}

impl<'b> minicbor::Decode<'b> for AuxiliaryData {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Map | minicbor::data::Type::MapIndef => {
                Ok(AuxiliaryData::Shelley(d.decode()?))
            }
            minicbor::data::Type::Array => {
                d.array()?;
                let transaction_metadata = d.decode()?;
                let auxiliary_scripts = d.decode()?;
                Ok(AuxiliaryData::ShelleyMa {
                    transaction_metadata,
                    auxiliary_scripts,
                })
            }
            minicbor::data::Type::Tag => {
                d.tag()?;
                Ok(AuxiliaryData::Alonzo(d.decode()?))
            }
            _ => Err(minicbor::decode::Error::message(
                "Can't infer variant from data type for AuxiliaryData",
            )),
        }
    }
}

impl minicbor::Encode for AuxiliaryData {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            AuxiliaryData::Shelley(m) => {
                e.encode(m)?;
            }
            AuxiliaryData::ShelleyMa {
                transaction_metadata,
                auxiliary_scripts,
            } => {
                e.array(2)?;
                e.encode(transaction_metadata)?;
                e.encode(auxiliary_scripts)?;
            }
            AuxiliaryData::Alonzo(v) => {
                // TODO: check if this is the correct tag
                e.tag(Tag::Unassigned(259))?;
                e.encode(v)?;
            }
        };

        Ok(())
    }
}

pub type TransactionIndex = u32;

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct Block {
    #[n(0)]
    pub header: Header,

    #[n(1)]
    pub transaction_bodies: MaybeIndefArray<TransactionBody>,

    #[n(2)]
    pub transaction_witness_sets: MaybeIndefArray<TransactionWitnessSet>,

    #[n(3)]
    pub auxiliary_data_set: KeyValuePairs<TransactionIndex, AuxiliaryData>,

    #[n(4)]
    pub invalid_transactions: Option<MaybeIndefArray<TransactionIndex>>,
}

#[derive(Encode, Decode, Debug)]
pub struct BlockWrapper(#[n(0)] pub u16, #[n(1)] pub Block);

#[cfg(test)]
mod tests {
    use super::BlockWrapper;
    use crate::Fragment;
    use pallas_codec::minicbor::to_vec;

    #[test]
    fn block_isomorphic_decoding_encoding() {
        let test_blocks = vec![
            include_str!("test_data/test1.block"),
            include_str!("test_data/test2.block"),
            include_str!("test_data/test3.block"),
            include_str!("test_data/test4.block"),
            include_str!("test_data/test5.block"),
            include_str!("test_data/test6.block"),
            include_str!("test_data/test7.block"),
            include_str!("test_data/test8.block"),
            include_str!("test_data/test9.block"),
            // old block without invalid_transactions fields
            include_str!("test_data/test10.block"),
            // peculiar block with protocol update params
            include_str!("test_data/test11.block"),
            // peculiar block with decoding issue
            // https://github.com/txpipe/oura/issues/37
            include_str!("test_data/test12.block"),
            // peculiar block with protocol update params, including nonce
            include_str!("test_data/test13.block"),
            // peculiar block with overflow crash
            // https://github.com/txpipe/oura/issues/113
            include_str!("test_data/test14.block"),
            // peculiar block with many move-instantaneous-rewards certs
            include_str!("test_data/test15.block"),
            // peculiar block with protocol update values
            include_str!("test_data/test16.block"),
            // peculiar block with missing nonce hash
            include_str!("test_data/test17.block"),
            // peculiar block with strange AuxiliaryData variant
            include_str!("test_data/test18.block"),
            // peculiar block with strange AuxiliaryData variant
            include_str!("test_data/test18.block"),
            // peculiar block with nevative i64 overflow
            include_str!("test_data/test19.block"),
            // peculiar block with very BigInt in plutus code
            include_str!("test_data/test20.block"),
            // peculiar block with bad tx hash
            include_str!("test_data/test21.block"),
        ];

        for (idx, block_str) in test_blocks.iter().enumerate() {
            println!("decoding test block {}", idx + 1);
            let bytes = hex::decode(block_str).expect(&format!("bad block file {}", idx));

            let block = BlockWrapper::decode_fragment(&bytes[..])
                .expect(&format!("error decoding cbor for file {}", idx));

            let bytes2 =
                to_vec(block).expect(&format!("error encoding block cbor for file {}", idx));

            assert_eq!(bytes, bytes2);
        }
    }
}
