//! Era-aware Cardano ledger types with their CBOR codecs.
//!
//! This is the data layer that the rest of the Pallas ledger crates sit on:
//! [`pallas-traverse`] gives you a multi-era read API over these types,
//! [`pallas-validate`] applies ledger rules to them, and [`pallas-txbuilder`]
//! builds new ones.
//!
//! If you need raw, era-specific access to a `Tx`, `Block`, or `PlutusData`,
//! you want this crate. If you'd rather work over many eras through one
//! interface, reach for [`pallas-traverse`].
//!
//! [`pallas-traverse`]: https://crates.io/crates/pallas-traverse
//! [`pallas-validate`]: https://crates.io/crates/pallas-validate
//! [`pallas-txbuilder`]: https://crates.io/crates/pallas-txbuilder
//!
//! # Usage
//!
//! ```no_run
//! use pallas_codec::minicbor;
//! use pallas_primitives::conway;
//!
//! # let cbor_bytes: Vec<u8> = vec![];
//! let tx: conway::Tx = minicbor::decode(&cbor_bytes)?;
//!
//! for input in tx.transaction_body.inputs.iter() {
//!     println!("{:?}#{}", input.transaction_id, input.index);
//! }
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
//!
//! # Overview
//!
//! - [`byron`], [`alonzo`], [`babbage`], [`conway`] — one module per era,
//!   each exposing the era's `Block`, `Tx`, `TransactionInput`,
//!   `TransactionOutput`, `Value`, `Certificate`, `Metadata`, witness sets,
//!   and so on.
//! - `plutus_data` — re-exported [`PlutusData`], [`BigInt`], and helpers
//!   shared across eras.
//! - `framework` — common type aliases and codec primitives
//!   ([`AddrKeyhash`], [`Coin`], [`PolicyId`], [`RationalNumber`],
//!   [`StakeCredential`], [`TransactionInput`], [`ExUnits`],
//!   [`PlutusScript`], …).
//! - Re-exports from [`pallas-codec`] ([`Bytes`], [`KeepRaw`],
//!   [`KeyValuePairs`], [`NonEmptySet`], [`Set`], [`Nullable`], …) and
//!   [`pallas-crypto`] ([`struct@Hash`]).
//!
//! [`pallas-codec`]: https://crates.io/crates/pallas-codec
//! [`pallas-crypto`]: https://crates.io/crates/pallas-crypto
//!
//! # Feature flags
//!
//! - `relaxed` — relax some validation invariants applied during decoding;
//!   useful for round-tripping non-canonical historical artifacts.
//!
//! # Usage as part of `pallas`
//!
//! When depending on the umbrella [`pallas`] crate, this crate is re-exported
//! as `pallas::ledger::primitives`.
//!
//! [`pallas`]: https://crates.io/crates/pallas

mod framework;
mod plutus_data;

/// Ledger primitives for the Alonzo era (smart contracts).
pub mod alonzo;
/// Ledger primitives for the Babbage era (reference inputs / inline datums).
pub mod babbage;
/// Ledger primitives for the Byron era.
pub mod byron;
/// Ledger primitives for the Conway era (governance).
pub mod conway;
pub use plutus_data::*;

pub use framework::*;

pub use pallas_codec::codec_by_datatype;

pub use pallas_codec::utils::{
    Bytes, Int, KeepRaw, KeyValuePairs, MaybeIndefArray, NonEmptySet, NonZeroInt, Nullable,
    PositiveCoin, Set,
};
pub use pallas_crypto::hash::Hash;

use pallas_codec::minicbor::{self, data::Tag, Decode, Encode};
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

// ----- Common type definitions

/// Hash of a Cardano address verification key (Blake2b-224).
pub type AddrKeyhash = Hash<28>;

/// Token name within a multi-asset bundle (raw bytes, up to 32 long).
pub type AssetName = Bytes;

/// Quantity in lovelace.
pub type Coin = u64;

/// Plutus cost model: ordered list of per-primitive cost coefficients.
pub type CostModel = Vec<i64>;

/// Hash of a Plutus datum (Blake2b-256).
pub type DatumHash = Hash<32>;

/// DNS name (A or SRV record) used in relay declarations.
pub type DnsName = String;

/// Epoch number on the Cardano chain.
pub type Epoch = u64;

/// Plutus script execution budget: memory and step units.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone, Copy)]
pub struct ExUnits {
    /// Memory units consumed.
    #[n(0)]
    pub mem: u64,
    /// CPU step units consumed.
    #[n(1)]
    pub steps: u64,
}

/// Per-unit prices used to convert [`ExUnits`] into fee lovelace.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct ExUnitPrices {
    /// Price per memory unit.
    #[n(0)]
    pub mem_price: PositiveInterval,

    /// Price per CPU step.
    #[n(1)]
    pub step_price: PositiveInterval,
}

/// Hash identifying a genesis configuration.
pub type Genesishash = Bytes;

/// Hash of a genesis delegate certificate.
pub type GenesisDelegateHash = Bytes;

/// IPv4 address bytes (4 bytes, big-endian).
pub type IPv4 = Bytes;

/// IPv6 address bytes (16 bytes, big-endian).
pub type IPv6 = Bytes;

/// Transaction metadata map, keyed by label.
pub type Metadata = BTreeMap<MetadatumLabel, Metadatum>;

/// Single metadata value of any supported CBOR shape.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Metadatum {
    /// Integer (signed or unsigned, up to 64 bits).
    Int(Int),
    /// Raw byte string.
    Bytes(Bytes),
    /// UTF-8 text string.
    Text(String),
    /// Ordered list of metadata values.
    Array(Vec<Metadatum>),
    /// Map of metadata values keyed by metadata values.
    Map(KeyValuePairs<Metadatum, Metadatum>),
}

codec_by_datatype! {
    Metadatum,
    U8 | U16 | U32 | U64 | I8 | I16 | I32 | I64 | Int => Int,
    Bytes => Bytes,
    String | StringIndef => Text,
    Array | ArrayIndef => Array,
    Map | MapIndef => Map,
    ()
}

/// Top-level metadata label (CIP-10 / CIP-25 / etc.).
pub type MetadatumLabel = u64;

/// The network this artifact targets (encoded as a small CBOR enum).
#[derive(
    Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy,
)]
#[cbor(index_only)]
pub enum NetworkId {
    /// The Cardano testnet.
    #[n(0)]
    Testnet,
    /// The Cardano mainnet.
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

/// Praos nonce used as input to the leader-selection schedule.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct Nonce {
    /// Discriminator selecting how the nonce was produced.
    #[n(0)]
    pub variant: NonceVariant,

    /// Hash payload, present when `variant` is [`NonceVariant::Nonce`].
    #[n(1)]
    pub hash: Option<Hash<32>>,
}

/// Discriminator for [`Nonce`]: neutral (genesis) or hashed.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(index_only)]
pub enum NonceVariant {
    /// Initial neutral nonce, with no hash payload.
    #[n(0)]
    NeutralNonce,

    /// A hashed nonce; the payload lives in [`Nonce::hash`].
    #[n(1)]
    Nonce,
}

/// Raw bytes of a Plutus script of language version `VERSION` (1, 2, or 3).
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cbor(transparent)]
pub struct PlutusScript<const VERSION: usize>(pub Bytes);

impl<const VERSION: usize> AsRef<[u8]> for PlutusScript<VERSION> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

/// Hash of a minting policy (Blake2b-224).
pub type PolicyId = Hash<28>;

/// Hash of a stake pool's cold key (Blake2b-224).
pub type PoolKeyhash = Hash<28>;

/// Stake pool metadata reference: URL plus the hash of the pointed-to JSON.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct PoolMetadata {
    /// URL serving the pool metadata JSON.
    #[n(0)]
    pub url: String,

    /// Hash of the JSON document served at `url`.
    #[n(1)]
    pub hash: PoolMetadataHash,
}

/// Hash of stake pool metadata (Blake2b-256).
pub type PoolMetadataHash = Bytes;

/// TCP/UDP port number.
pub type Port = u32;

/// Rational number guaranteed to be strictly positive.
pub type PositiveInterval = RationalNumber;

/// Protocol version: `(major, minor)`.
pub type ProtocolVersion = (u64, u64);

/// On-chain rational number, encoded as a CBOR tag-30 array of `[num, den]`.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct RationalNumber {
    /// Numerator of the rational.
    pub numerator: u64,
    /// Denominator of the rational.
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

/// Network endpoint declared by a stake pool's relay.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Relay {
    /// IP-based relay with optional port and either or both IPv4/IPv6.
    SingleHostAddr(Option<Port>, Option<IPv4>, Option<IPv6>),
    /// DNS A-record relay with optional port and a hostname.
    SingleHostName(Option<Port>, DnsName),
    /// DNS SRV-record relay (port and host both come from the SRV record).
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

/// Reward-account bytes (network header byte plus stake-credential hash).
pub type RewardAccount = Bytes;

/// Hash of a script (Blake2b-224).
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
/// On-chain credential controlling a stake address: a script or a key hash.
pub enum StakeCredential {
    /// Stake credential backed by a script hash.
    #[n(1)]
    ScriptHash(#[n(0)] ScriptHash),
    /// Stake credential backed by a verification-key hash.
    #[n(0)]
    AddrKeyhash(#[n(0)] AddrKeyhash),
}

/// Index of a transaction within its containing block.
pub type TransactionIndex = u32;

/// Reference to a transaction output: `(tx_hash, output_index)`.
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
    /// Hash of the transaction that produced the output.
    #[n(0)]
    pub transaction_id: Hash<32>,

    /// Index of the output within that transaction.
    #[n(1)]
    pub index: u64,
}

/// Rational number constrained to the closed interval [0, 1].
pub type UnitInterval = RationalNumber;

/// VRF certificate: the output bytes followed by the proof bytes.
#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct VrfCert(
    /// VRF output bytes.
    #[n(0)]
    pub Bytes,
    /// VRF proof bytes.
    #[n(1)]
    pub Bytes,
);

/// Hash of a VRF verification key (Blake2b-256).
pub type VrfKeyhash = Hash<32>;
