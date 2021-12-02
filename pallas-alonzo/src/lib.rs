//! Ledger primitives and cbor codec for the Alonzo era
//!
//! Handcrafted, idiomatic rust artifacts based on based on the [Alonzo CDDL](https://github.com/input-output-hk/cardano-ledger/blob/master/eras/alonzo/test-suite/cddl-files/alonzo.cddl) file in IOHK repo.

use log::warn;
use minicbor::{bytes::ByteVec, data::Tag};
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

#[derive(Debug, PartialEq, Clone)]
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
}
