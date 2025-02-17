//! Ledger primitives and cbor codec for the Cardano eras

#[macro_use]
mod framework;
mod plutus_data;

pub mod alonzo;
pub mod babbage;
pub mod byron;
pub mod conway;
pub use plutus_data::*;

pub use framework::*;

pub use pallas_codec::utils::{
    Bytes, Int, KeepRaw, KeyValuePairs, MaybeIndefArray, NonEmptyKeyValuePairs, NonEmptySet,
    NonZeroInt, Nullable, PositiveCoin, Set,
};
pub use pallas_crypto::hash::Hash;

use pallas_codec::minicbor::{self, data::Tag, Decode, Encode};
use serde::{Deserialize, Serialize};

// ----- Common type definitions

pub type AddrKeyhash = Hash<28>;

pub type AssetName = Bytes;

pub type Coin = u64;

pub type CostModel = Vec<i64>;

pub type DatumHash = Hash<32>;

pub type DnsName = String;

pub type Epoch = u64;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone, Copy)]
pub struct ExUnits {
    #[n(0)]
    pub mem: u64,
    #[n(1)]
    pub steps: u64,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct ExUnitPrices {
    #[n(0)]
    pub mem_price: PositiveInterval,

    #[n(1)]
    pub step_price: PositiveInterval,
}

pub type Genesishash = Bytes;

pub type GenesisDelegateHash = Bytes;

pub type IPv4 = Bytes;

pub type IPv6 = Bytes;

pub type Metadata = KeyValuePairs<MetadatumLabel, Metadatum>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Metadatum {
    Int(Int),
    Bytes(Bytes),
    Text(String),
    Array(Vec<Metadatum>),
    Map(KeyValuePairs<Metadatum, Metadatum>),
}

// Missing StringIndef?
codec_by_datatype! {
    Metadatum,
    Int => U8 | U16 | U32 | U64 | I8 | I16 | I32 | I64 | Int,
    Bytes => Bytes,
    Text => String,
    Array => Array | ArrayIndef,
    Map => Map | MapIndef,
    ()
}

pub type MetadatumLabel = u64;

#[derive(
    Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy,
)]
#[cbor(index_only)]
pub enum NetworkId {
    #[n(0)]
    Testnet,
    #[n(1)]
    Mainnet,
}

impl From<NetworkId> for u8 {
    fn from(network_id: NetworkId) -> u8 {
        match network_id {
            NetworkId::Testnet => 0,
            NetworkId::Mainnet => 1,
        }
    }
}

impl TryFrom<u8> for NetworkId {
    type Error = ();

    fn try_from(i: u8) -> Result<Self, Self::Error> {
        match i {
            0 => Ok(Self::Testnet),
            1 => Ok(Self::Mainnet),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Nonce {
    #[n(0)]
    pub variant: NonceVariant,

    #[n(1)]
    pub hash: Option<Hash<32>>,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(index_only)]
pub enum NonceVariant {
    #[n(0)]
    NeutralNonce,

    #[n(1)]
    Nonce,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct PlutusScript<const VERSION: usize>(#[n(0)] pub Bytes);

impl<const VERSION: usize> AsRef<[u8]> for PlutusScript<VERSION> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

pub type PolicyId = Hash<28>;

pub type PoolKeyhash = Hash<28>;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct PoolMetadata {
    #[n(0)]
    pub url: String,

    #[n(1)]
    pub hash: PoolMetadataHash,
}

pub type PoolMetadataHash = Hash<32>;

pub type Port = u32;

pub type PositiveInterval = RationalNumber;

pub type ProtocolVersion = (u64, u64);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct RationalNumber {
    pub numerator: u64,
    pub denominator: u64,
}

impl<'b, C> minicbor::decode::Decode<'b, C> for RationalNumber {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        // TODO: Enforce tag == 30 & array of size 2
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
        e.tag(Tag::new(30))?;
        e.array(2)?;
        e.encode_with(self.numerator, ctx)?;
        e.encode_with(self.denominator, ctx)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Relay {
    SingleHostAddr(Nullable<Port>, Nullable<IPv4>, Nullable<IPv6>),
    SingleHostName(Nullable<Port>, DnsName),
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

pub type RewardAccount = Bytes;

pub type ScriptHash = Hash<28>;

#[derive(
    Serialize, Deserialize, Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash, Encode, Decode,
)]
// !! NOTE / IMPORTANT !!
// It is tempting to swap the order of the two constructors so that AddrKeyHash
// comes first. This indeed nicely maps the binary representation which
// associates 0 to AddrKeyHash and 1 to ScriptHash.
//
// However, for historical reasons, the ScriptHash variant comes first in the
// Haskell reference codebase. From this ordering is derived the `PartialOrd`
// and `Ord` instances; which impacts how Maps/Dictionnaries indexed by
// StakeCredential will be ordered. So, it is crucial to preserve this quirks to
// avoid hard to troubleshoot issues down the line.
#[cbor(flat)]
pub enum StakeCredential {
    #[n(1)]
    ScriptHash(#[n(0)] ScriptHash),
    #[n(0)]
    AddrKeyhash(#[n(0)] AddrKeyhash),
}

pub type TransactionIndex = u32;

#[derive(
    Serialize,
    Deserialize,
    Encode,
    Decode,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    std::hash::Hash,
)]
pub struct TransactionInput {
    #[n(0)]
    pub transaction_id: Hash<32>,

    #[n(1)]
    pub index: u64,
}

pub type UnitInterval = RationalNumber;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct VrfCert(#[n(0)] pub Bytes, #[n(1)] pub Bytes);

pub type VrfKeyhash = Hash<32>;
