//! Ledger primitives and cbor codec for the Alonzo era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Alonzo CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/alonzo/test-suite/cddl-files/alonzo.cddl) file in IOHK repo.

use log::{log_enabled, warn};
use minicbor::{bytes::ByteVec, data::Tag, Decode, Encode};
use minicbor_derive::{Decode, Encode};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct SkipCbor<const N: usize> {}

impl<'b, const N: usize> minicbor::Decode<'b> for SkipCbor<N> {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        {
            let probe = d.probe();
            warn!("skipped cbor value {}: {:?}", N, probe.datatype()?);
        }

        d.skip()?;
        Ok(SkipCbor {})
    }
}

impl<const N: usize> minicbor::Encode for SkipCbor<N> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        todo!()
    }
}

pub type SomeSkipCbor = SkipCbor<0>;

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct VrfCert(#[n(0)] ByteVec, #[n(1)] ByteVec);

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct HeaderBody {
    #[n(0)]
    pub block_number: u64,

    #[n(1)]
    pub slot: u64,

    #[n(2)]
    pub prev_hash: ByteVec,

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
    pub block_body_hash: ByteVec,

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

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct Header {
    #[n(0)]
    pub header_body: HeaderBody,

    #[n(1)]
    pub body_signature: ByteVec,
}

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct TransactionInput {
    #[n(0)]
    pub transaction_id: ByteVec,

    #[n(1)]
    pub index: u64,
}

pub type ScriptHash = ByteVec;

pub type PolicyId = ScriptHash;

pub type AssetName = ByteVec;

pub type Multiasset<A> = HashMap<PolicyId, HashMap<AssetName, A>>;

pub type Mint = Multiasset<i64>;

pub type Coin = u64;

#[derive(Debug, PartialEq)]
pub enum Value {
    Coin(Coin),
    Multiasset(Coin, Multiasset<u64>),
}

impl<'b> minicbor::decode::Decode<'b> for Value {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::U32 => Ok(Value::Coin(d.decode()?)),
            minicbor::data::Type::U64 => Ok(Value::Coin(d.decode()?)),
            minicbor::data::Type::Array => {
                d.array()?;
                let coin = d.u64()?;
                let multiasset = d.decode()?;
                Ok(Value::Multiasset(coin, multiasset))
            }
            _ => Err(minicbor::decode::Error::Message(
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
        todo!()
    }
}

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct TransactionOutput {
    #[n(0)]
    pub address: ByteVec,

    #[n(1)]
    pub amount: Value,

    #[n(2)]
    pub datum_hash: Option<ByteVec>,
}

pub type Hash28 = ByteVec;
pub type Hash32 = ByteVec;

pub type PoolKeyhash = Hash28;
pub type Epoch = i64;
pub type Genesishash = SkipCbor<5>;
pub type GenesisDelegateHash = SkipCbor<6>;
pub type VrfKeyhash = Hash32;
pub type MoveInstantaneousReward = SkipCbor<8>;
pub type Margin = SkipCbor<9>;
pub type RewardAccount = ByteVec;
pub type PoolOwners = SkipCbor<11>;

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
            _ => Err(minicbor::decode::Error::Message(
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
        todo!()
    }
}

pub type PoolMetadataHash = Hash32;

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct PoolMetadata {
    #[n(0)]
    url: String,

    #[n(1)]
    hash: PoolMetadataHash,
}

pub type AddrKeyhash = Hash28;
pub type Scripthash = Hash28;

#[derive(Debug, PartialEq)]
pub struct RationalNumber {
    numerator: i64,
    denominator: u64,
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
        todo!()
    }
}

pub type UnitInterval = RationalNumber;

#[derive(Debug, PartialEq)]
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
            _ => Err(minicbor::decode::Error::Message(
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
        todo!()
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
        pool_owners: Vec<AddrKeyhash>,
        relays: Vec<Relay>,
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
            _ => Err(minicbor::decode::Error::Message(
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
        todo!()
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
    pub withdrawals: Option<BTreeMap<RewardAccount, Coin>>,

    #[n(6)]
    pub update: Option<SkipCbor<22>>,

    #[n(7)]
    pub auxiliary_data_hash: Option<ByteVec>,

    #[n(8)]
    pub validity_interval_start: Option<u64>,

    #[n(9)]
    pub mint: Option<Multiasset<i64>>,

    #[n(11)]
    pub script_data_hash: Option<Hash32>,

    #[n(13)]
    pub collateral: Option<Vec<TransactionInput>>,

    #[n(14)]
    pub required_signers: Option<Vec<AddrKeyhash>>,

    #[n(15)]
    pub network_id: Option<NetworkId>,
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
    ScriptAll(Vec<NativeScript>),
    ScriptAny(Vec<NativeScript>),
    ScriptNOfK(u32, Vec<NativeScript>),
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
            _ => Err(minicbor::decode::Error::Message(
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
        todo!()
    }
}

pub type PlutusScript = ByteVec;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlutusData {
    Constr(Constr<PlutusData>),
    Map(BTreeMap<PlutusData, PlutusData>),
    BitInt(u64),
    BoundedBytes(ByteVec),
    Array(Vec<PlutusData>),
}

impl<'b> minicbor::decode::Decode<'b> for PlutusData {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let type_ = d.datatype()?;

        match type_ {
            minicbor::data::Type::Tag => Ok(PlutusData::Constr(d.decode()?)),
            minicbor::data::Type::Map => Ok(PlutusData::Map(d.decode()?)),
            minicbor::data::Type::I8 => Ok(PlutusData::BitInt(d.decode()?)),
            minicbor::data::Type::I16 => Ok(PlutusData::BitInt(d.decode()?)),
            minicbor::data::Type::I32 => Ok(PlutusData::BitInt(d.decode()?)),
            minicbor::data::Type::I64 => Ok(PlutusData::BitInt(d.decode()?)),
            minicbor::data::Type::U8 => Ok(PlutusData::BitInt(d.decode()?)),
            minicbor::data::Type::U16 => Ok(PlutusData::BitInt(d.decode()?)),
            minicbor::data::Type::U32 => Ok(PlutusData::BitInt(d.decode()?)),
            minicbor::data::Type::U64 => Ok(PlutusData::BitInt(d.decode()?)),
            minicbor::data::Type::Bytes => Ok(PlutusData::BoundedBytes(d.decode()?)),
            minicbor::data::Type::Array => Ok(PlutusData::Array(d.decode()?)),
            minicbor::data::Type::ArrayIndef => Ok(PlutusData::Array(d.decode()?)),
            _ => Err(minicbor::decode::Error::Message(
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
        todo!()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Constr<A> {
    pub tag: u64,
    pub prefix: Option<u32>,
    pub values: Vec<A>,
}

impl<'b, A> minicbor::decode::Decode<'b> for Constr<A>
where
    A: minicbor::decode::Decode<'b>,
{
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let tag = d.tag()?;

        match tag {
            Tag::Unassigned(x) => match x {
                121 | 122 | 123 | 124 | 125 | 126 | 127 => Ok(Constr {
                    tag: x,
                    values: d.decode()?,
                    prefix: None,
                }),
                102 => {
                    d.array()?;
                    let prefix = Some(d.decode()?);
                    let values = d.decode()?;
                    Ok(Constr {
                        tag: 102,
                        prefix,
                        values,
                    })
                }
                _ => Err(minicbor::decode::Error::Message(
                    "bad tag code for plutus data",
                )),
            },
            _ => Err(minicbor::decode::Error::Message(
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
        todo!()
    }
}

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct ExUnits {
    #[n(0)]
    mem: u32,
    #[n(1)]
    steps: u32,
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
    tag: RedeemerTag,

    #[n(1)]
    index: u32,

    #[n(2)]
    data: PlutusData,

    #[n(3)]
    ex_units: ExUnits,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[cbor(map)]
pub struct TransactionWitnessSet {
    #[n(0)]
    pub vkeywitness: Option<Vec<VKeyWitness>>,

    #[n(1)]
    pub native_script: Option<Vec<NativeScript>>,

    #[n(2)]
    pub bootstrap_witness: Option<Vec<SkipCbor<32>>>,

    #[n(3)]
    pub plutus_script: Option<Vec<PlutusScript>>,

    #[n(4)]
    pub plutus_data: Option<Vec<PlutusData>>,

    #[n(5)]
    pub redeemer: Option<Vec<Redeemer>>,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[cbor(map)]
pub struct AlonzoAuxiliaryData {
    #[n(0)]
    pub metadata: Option<Metadata>,
    #[n(1)]
    pub native_scripts: Option<Vec<NativeScript>>,
    #[n(2)]
    pub plutus_scripts: Option<PlutusScript>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Metadatum {
    Int(i64),
    Bytes(ByteVec),
    Text(String),
    Array(Vec<Metadatum>),
    Map(BTreeMap<Metadatum, Metadatum>),
}

impl<'b> minicbor::Decode<'b> for Metadatum {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::U8 => {
                let i = d.u8()?;
                Ok(Metadatum::Int(i as i64))
            }
            minicbor::data::Type::U16 => {
                let i = d.u16()?;
                Ok(Metadatum::Int(i as i64))
            }
            minicbor::data::Type::U32 => {
                let i = d.u32()?;
                Ok(Metadatum::Int(i as i64))
            }
            minicbor::data::Type::U64 => {
                let i = d.u64()?;
                Ok(Metadatum::Int(i as i64))
            }
            minicbor::data::Type::I8 => {
                let i = d.i8()?;
                Ok(Metadatum::Int(i as i64))
            }
            minicbor::data::Type::I16 => {
                let i = d.i16()?;
                Ok(Metadatum::Int(i as i64))
            }
            minicbor::data::Type::I32 => {
                let i = d.i32()?;
                Ok(Metadatum::Int(i as i64))
            }
            minicbor::data::Type::I64 => {
                let i = d.i64()?;
                Ok(Metadatum::Int(i as i64))
            }
            minicbor::data::Type::Bytes => Ok(Metadatum::Bytes(d.decode()?)),
            minicbor::data::Type::String => Ok(Metadatum::Text(d.decode()?)),
            minicbor::data::Type::Array => Ok(Metadatum::Array(d.decode()?)),
            minicbor::data::Type::Map => Ok(Metadatum::Map(d.decode()?)),
            _ => Err(minicbor::decode::Error::Message(
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
        todo!()
    }
}

pub type Metadata = BTreeMap<Metadatum, Metadatum>;

#[derive(Debug, PartialEq)]
pub enum AuxiliaryData {
    Shelley(Metadata),
    ShelleyMa {
        transaction_metadata: Metadata,
        auxiliary_scripts: Vec<SomeSkipCbor>,
    },
    Alonzo(AlonzoAuxiliaryData),
}

impl<'b> minicbor::Decode<'b> for AuxiliaryData {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Map => Ok(AuxiliaryData::Shelley(d.decode()?)),
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
            _ => Err(minicbor::decode::Error::Message(
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
        todo!()
    }
}

pub type TransactionIndex = u32;

#[derive(Encode, Decode, Debug, PartialEq)]
pub struct Block {
    #[n(0)]
    pub header: Header,

    #[n(1)]
    pub transaction_bodies: Vec<TransactionBody>,

    #[n(2)]
    pub transaction_witness_sets: Vec<TransactionWitnessSet>,

    #[n(3)]
    pub auxiliary_data_set: HashMap<TransactionIndex, AuxiliaryData>,

    #[n(4)]
    pub invalid_transactions: Vec<TransactionIndex>,
}

impl TryFrom<&[u8]> for Block {
    type Error = minicbor::decode::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        // Hack to unwrap root array on top of the block. Can't find spec explaining
        // what this value means.
        let (_unknown, block): (u16, Block) = minicbor::decode(value)?;
        Ok(block)
    }
}

impl TryFrom<&[u8]> for Header {
    type Error = minicbor::decode::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        minicbor::decode(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Block, Header};

    const TEST_HEADER: &str = "828F1A006215E21A029D10A85820FF8D558A3D5A0E058BEB3D94D26A567F75CD7D09FF5485AA0D0EBC38B61378D458207A353CDFA0A5B5471BF05156097A4304DE6C9379E2937ACA6A00F954A2DC55CC582012DA72FCEAA4D03F6F409AF2AD2521C305082406B5AC24CBEE77CA3973B387BE825840988C010910CF4704EFF43B40F9B03868EF463546D942B8742BB4CFA78EB3E504BEAB8559D15F1D7F441BFED8265DACFCD96EE0D6744F08C4D4BFC07EF65A363D58508A66A6753E76E26285C30E18AC08EC683DE384A0EC710C94255A3392C1A500EB4A2737DE258640ADCB3B7CF98C431D2226DC5EFB8F61C941D5EA05469463396AF349FAB2AFF61E9FED64C5FE897651018258400000E31D3B969A44D385CFF79B9B0494675DF6A219438D551AD416BA2435216E3A712D3003EA97B638FEA979FA4405413694032760657D853882E8039071653F5850DCD355FC6F70D1D209EFC9120C9237A1F47EA3840473676766DA9867DF64058C7CB561466752D01E5439CBCA72EED20E042EFA9FE3167AD1A787D9D44A1219378F8BB1C62274C9246CF4BC99F4515304190A2F5820015D41B6EC5AC1DC88155B344E45B5A8D098C334CB997D3AC82A0571DAB5064558203BF4F4A527C0D227189F919893417AD461B8529E71BBD27E4ADE1EE20E8295F20519011558404F669665A14F8608608D848E3E8E61A9C17AFB55B4905C4ED3F3AC3152F0C8C9BA41ECE8607D240C33F5F2AE26FEFCF8422710E94261EA8152FBB7DC48BC0D0706005901C0B7F86863A0AA72E1693DC817A59F35104D5F26139CFAAB4F2EB7F39CA65B7BF31A0B52BA849DA4382369872E542585F153D888FA71799596E0CCF507074362066C4ED9F18E9F7D692487BC23679F561F5053A6AE0D79B3DA2C3C1DAF6FEAA8D8599417B409ABE17407EC339FEBFDBF4956C37925EE6448F44FE6AA0E715CC27E973CF28D21F1F5A40708BA7C229D282F152B95D6C7AEB9944E914FC68AFC1491B7DD883509A9D32F8B32FD078D1C0ACDEF06B4DF2908C07CC75405AE5B7C3829B92917E4B2A284F9ACEE94F2E17958AAC21EF0AC4E0D891081BCA96E0CE6907B5BEF9092340F5D6D9F249B02FC28024F7CF7560292B140FC656AD70BF915B4739092FF52CE23721E12340416AA9A22CC53801D98FB283873936609DFAA9A7F16028A2CE69FDF40F905E18DFE3148BB7B0B912B0FACD2F73B56E2EA7288C877356F36F0CFE2C0DD45A1C0C41F461A2AF154853F309BA97D1CF3AF539DDCB457DEE6032F2CE5E44638B56A60391DFBC50C3F05D86690B5B652FB45AC7ED62B06099B54257A6E0BDB91164172D4635BF2F3ABE85015414FAC2E7CBED797DF9211531422CA3F66B24FBE71A970649CCDD77DFF498AF535E38B7365A5DF0F9C12836C";

    #[test]
    fn block_decode_works() {
        let test_blocks = vec![
            include_str!("test_data/test1.block"),
            include_str!("test_data/test2.block"),
            include_str!("test_data/test3.block"),
            include_str!("test_data/test4.block"),
            include_str!("test_data/test5.block"),
            include_str!("test_data/test6.block"),
            include_str!("test_data/test7.block"),
            include_str!("test_data/test8.block"),
        ];

        for (idx, block_str) in test_blocks.iter().enumerate() {
            println!("decoding test block {}", idx + 1);
            let bytes = hex::decode(block_str).unwrap();
            Block::try_from(&bytes[..]).unwrap();
        }
    }

    #[test]
    fn header_decode_works() {
        let bytes = hex::decode(TEST_HEADER).unwrap();
        let decoded = Header::try_from(&bytes[..]).unwrap();

        assert_eq!(decoded.header_body.block_number, 6428130);
        assert_eq!(decoded.header_body.slot, 43847848);
        assert_eq!(decoded.header_body.protocol_version_major, 6);
        assert_eq!(decoded.header_body.protocol_version_minor, 0);
    }
}
