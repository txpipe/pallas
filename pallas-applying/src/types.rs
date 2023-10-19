//! Base types used for validating transactions in each era.

use std::collections::HashMap;

pub use pallas_traverse::{MultiEraInput, MultiEraOutput};

pub type UTxOs<'b> = HashMap<MultiEraInput<'b>, MultiEraOutput<'b>>;

// TODO: add a field for each protocol parameter in the Byron era.
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

// TODO: replace this generic variant with validation-rule-specific ones.
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
}

pub type ValidationResult = Result<(), ValidationError>;
