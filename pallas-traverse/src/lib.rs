//! Utilities to traverse over multi-era block data

use std::borrow::Cow;
use std::fmt::Display;

use pallas_primitives::{alonzo, byron};
use thiserror::Error;

pub mod block;
pub mod cert;
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
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MultiEraBlock<'b> {
    EpochBoundary(Cow<'b, byron::EbBlock>),
    AlonzoCompatible(Cow<'b, alonzo::MintedBlock<'b>>, Era),
    Byron(Cow<'b, byron::MintedBlock<'b>>),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraTx<'b> {
    AlonzoCompatible(Cow<'b, alonzo::MintedTx<'b>>),
    Byron(Cow<'b, byron::MintedTxPayload<'b>>),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraOutput<'b> {
    Byron(Cow<'b, byron::TxOut>),
    AlonzoCompatible(Cow<'b, alonzo::TransactionOutput>),
}

pub enum MultiEraCert<'b> {
    NotApplicable,
    AlonzoCompatible(Cow<'b, alonzo::Certificate>),
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
