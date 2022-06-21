//! Utilities to traverse over multi-era block data

use std::borrow::Cow;
use std::fmt::Display;

use thiserror::Error;

use pallas_codec::utils::KeepRaw;
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, babbage, byron};

pub mod block;
pub mod cert;
pub mod era;
pub mod header;
pub mod input;
pub mod output;
pub mod probe;
mod support;
pub mod tx;

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
    EpochBoundary(KeepRaw<'b, byron::EbbHead>),
    AlonzoCompatible(KeepRaw<'b, alonzo::Header>),
    Babbage(KeepRaw<'b, babbage::Header>),
    Byron(KeepRaw<'b, byron::BlockHead>),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraBlock<'b> {
    EpochBoundary(Box<Cow<'b, byron::EbBlock>>),
    AlonzoCompatible(Box<Cow<'b, alonzo::MintedBlock<'b>>>, Era),
    Babbage(Box<Cow<'b, babbage::MintedBlock<'b>>>),
    Byron(Box<Cow<'b, byron::MintedBlock<'b>>>),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraTx<'b> {
    AlonzoCompatible(Box<Cow<'b, alonzo::MintedTx<'b>>>, Era),
    Babbage(Box<Cow<'b, babbage::MintedTx<'b>>>),
    Byron(Box<Cow<'b, byron::MintedTxPayload<'b>>>),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraOutput<'b> {
    AlonzoCompatible(Box<Cow<'b, alonzo::TransactionOutput>>),
    Babbage(Box<Cow<'b, babbage::TransactionOutput>>),
    Byron(Box<Cow<'b, byron::TxOut>>),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraInput<'b> {
    Byron(Box<Cow<'b, byron::TxIn>>),
    AlonzoCompatible(Box<Cow<'b, alonzo::TransactionInput>>),
}

pub enum MultiEraCert<'b> {
    NotApplicable,
    AlonzoCompatible(Box<Cow<'b, alonzo::Certificate>>),
}

pub struct OutputRef<'a>(Cow<'a, Hash<32>>, u64);

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
}

impl Error {
    pub fn invalid_cbor(error: impl Display) -> Self {
        Error::InvalidCbor(format!("{}", error))
    }

    pub fn unknown_cbor(bytes: &[u8]) -> Self {
        Error::UnknownCbor(hex::encode(bytes))
    }
}
