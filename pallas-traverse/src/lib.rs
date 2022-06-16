//! Utilities to traverse over multi-era block data

use std::borrow::Cow;
use std::fmt::Display;

use pallas_primitives::{alonzo, byron};
use thiserror::Error;

pub mod block;
pub mod cert;
pub mod era;
pub mod output;
pub mod input;
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
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Feature {
    TimeLocks,
    MultiAssets,
    Staking,
    SmartContracts,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraBlock<'b> {
    EpochBoundary(Box<Cow<'b, byron::EbBlock>>),
    AlonzoCompatible(Box<Cow<'b, alonzo::MintedBlock<'b>>>, Era),
    Byron(Box<Cow<'b, byron::MintedBlock<'b>>>),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraTx<'b> {
    AlonzoCompatible(Box<Cow<'b, alonzo::MintedTx<'b>>>),
    Byron(Box<Cow<'b, byron::MintedTxPayload<'b>>>),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraOutput<'b> {
    Byron(Box<Cow<'b, byron::TxOut>>),
    AlonzoCompatible(Box<Cow<'b, alonzo::TransactionOutput>>),
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

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid CBOR structure: {0}")]
    InvalidCbor(String),

    #[error("Unknown CBOR structure: {0}")]
    UnknownCbor(String),
}

impl Error {
    pub fn invalid_cbor(error: impl Display) -> Self {
        Error::InvalidCbor(format!("{}", error))
    }

    pub fn unknown_cbor(bytes: &[u8]) -> Self {
        Error::UnknownCbor(hex::encode(bytes))
    }
}
