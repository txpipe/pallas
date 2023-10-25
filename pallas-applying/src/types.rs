//! Base types used for validating transactions in each era.

use std::collections::HashMap;

pub use pallas_traverse::{MultiEraInput, MultiEraOutput};

pub type UTxOs<'b> = HashMap<MultiEraInput<'b>, MultiEraOutput<'b>>;

#[derive(Debug, Clone)]
pub struct ByronProtParams {
    pub min_fees_const: u64,
    pub min_fees_factor: u64,
    pub max_tx_size: u64,
}

// TODO: add variants for the other eras.
#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraProtParams {
    Byron(ByronProtParams),
}

pub struct Environment {
    pub prot_params: MultiEraProtParams,
    pub prot_magic: u32,
}

#[non_exhaustive]
pub enum SigningTag {
    Tx = 0x01,
    RedeemTx = 0x02,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ValidationError {
    InputMissingInUTxO,
    TxInsEmpty,
    TxOutsEmpty,
    OutputWithoutLovelace,
    UnknownTxSize,
    UnableToComputeFees,
    FeesBelowMin,
    MaxTxSizeExceeded,
    UnableToProcessWitnesses,
    MissingWitness,
    WrongSignature,
}

pub type ValidationResult = Result<(), ValidationError>;
