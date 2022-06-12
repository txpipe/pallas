//! Utilities to traverse over multi-era block data
use std::fmt::Display;

use pallas_primitives::{alonzo, byron};
use thiserror::Error;

pub mod block;
pub mod iter;
pub mod probe;
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

#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraTx<'b> {
    AlonzoCompatible(Box<alonzo::MintedTx<'b>>),
    Byron(Box<byron::MintedTxPayload<'b>>),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraBlock<'b> {
    EpochBoundary(Box<byron::EbBlock>),
    AlonzoCompatible(Box<alonzo::MintedBlock<'b>>),
    Byron(Box<byron::MintedBlock<'b>>),
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
