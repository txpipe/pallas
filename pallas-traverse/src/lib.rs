//! Utilities to traverse over multi-era block data

use std::{borrow::Cow, fmt::Display, hash::Hash as StdHash};

use thiserror::Error;

use pallas_codec::utils::{KeepRaw, KeyValuePairs};
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, babbage, byron};

mod support;

pub mod assets;
pub mod auxiliary;
pub mod block;
pub mod cert;
pub mod era;
pub mod fees;
pub mod hashes;
pub mod header;
pub mod input;
pub mod meta;
pub mod output;
pub mod probe;
pub mod signers;
pub mod size;
pub mod time;
pub mod tx;
pub mod withdrawals;
pub mod witnesses;

// TODO: move to genesis crate
pub mod wellknown;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Era {
    Byron,
    Shelley,
    Allegra, // time-locks
    Mary,    // multi-assets
    Alonzo,  // smart-contracts
    Babbage, // CIP-31/32/33
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Feature {
    TimeLocks,
    MultiAssets,
    Staking,
    SmartContracts,
    CIP31,
    CIP32,
    CIP33,
}

#[derive(Debug)]
pub enum MultiEraHeader<'b> {
    EpochBoundary(Cow<'b, KeepRaw<'b, byron::EbbHead>>),
    AlonzoCompatible(Cow<'b, KeepRaw<'b, alonzo::Header>>),
    Babbage(Cow<'b, KeepRaw<'b, babbage::Header>>),
    Byron(Cow<'b, KeepRaw<'b, byron::BlockHead>>),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraBlock<'b> {
    EpochBoundary(Box<byron::MintedEbBlock<'b>>),
    AlonzoCompatible(Box<alonzo::MintedBlock<'b>>, Era),
    Babbage(Box<babbage::MintedBlock<'b>>),
    Byron(Box<byron::MintedBlock<'b>>),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraTx<'b> {
    AlonzoCompatible(Box<Cow<'b, alonzo::MintedTx<'b>>>, Era),
    Babbage(Box<Cow<'b, babbage::MintedTx<'b>>>),
    Byron(Box<Cow<'b, byron::MintedTxPayload<'b>>>),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraOutput<'b> {
    AlonzoCompatible(Box<Cow<'b, alonzo::TransactionOutput>>),
    Babbage(Box<Cow<'b, babbage::MintedTransactionOutput<'b>>>),
    Byron(Box<Cow<'b, byron::TxOut>>),
}

#[derive(Debug, Clone, PartialEq, Eq, StdHash)]
#[non_exhaustive]
pub enum MultiEraInput<'b> {
    Byron(Box<Cow<'b, byron::TxIn>>),
    AlonzoCompatible(Box<Cow<'b, alonzo::TransactionInput>>),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraCert<'b> {
    NotApplicable,
    AlonzoCompatible(Box<Cow<'b, alonzo::Certificate>>),
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum MultiEraMeta<'b> {
    #[default]
    Empty,
    NotApplicable,
    AlonzoCompatible(&'b alonzo::Metadata),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraPolicyAssets<'b> {
    AlonzoCompatibleMint(
        &'b alonzo::PolicyId,
        &'b KeyValuePairs<alonzo::AssetName, i64>,
    ),
    AlonzoCompatibleOutput(
        &'b alonzo::PolicyId,
        &'b KeyValuePairs<alonzo::AssetName, u64>,
    ),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraAsset<'b> {
    AlonzoCompatibleOutput(&'b alonzo::PolicyId, &'b alonzo::AssetName, u64),
    AlonzoCompatibleMint(&'b alonzo::PolicyId, &'b alonzo::AssetName, i64),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraWithdrawals<'b> {
    NotApplicable,
    Empty,
    AlonzoCompatible(&'b alonzo::Withdrawals),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraSigners<'b> {
    NotApplicable,
    Empty,
    AlonzoCompatible(&'b alonzo::RequiredSigners),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct OutputRef(Hash<32>, u64);

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid CBOR structure: {0}")]
    InvalidCbor(String),

    #[error("Unknown CBOR structure: {0}")]
    UnknownCbor(String),

    #[error("Unknown era tag: {0}")]
    UnknownEra(u16),

    #[error("Invalid era for request: {0}")]
    InvalidEra(Era),

    #[error("Invalid UTxO ref: {0}")]
    InvalidUtxoRef(String),
}

impl Error {
    pub fn invalid_cbor(error: impl Display) -> Self {
        Error::InvalidCbor(format!("{error}"))
    }

    pub fn unknown_cbor(bytes: &[u8]) -> Self {
        Error::UnknownCbor(hex::encode(bytes))
    }

    pub fn invalid_utxo_ref(str: &str) -> Self {
        Error::InvalidUtxoRef(str.to_owned())
    }
}

pub trait ComputeHash<const BYTES: usize> {
    fn compute_hash(&self) -> pallas_crypto::hash::Hash<BYTES>;
}

pub trait OriginalHash<const BYTES: usize> {
    fn original_hash(&self) -> pallas_crypto::hash::Hash<BYTES>;
}
