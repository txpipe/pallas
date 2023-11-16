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

#[derive(Debug, Clone)]
pub struct ShelleyProtParams;

// TODO: add variants for the other eras.
#[derive(Debug)]
#[non_exhaustive]
pub enum MultiEraProtParams {
    Byron(ByronProtParams),
    Shelley(ShelleyProtParams),
}

#[derive(Debug)]
pub struct Environment {
    pub prot_params: MultiEraProtParams,
    pub prot_magic: u32,
    pub block_slot: u64,
}

#[non_exhaustive]
pub enum SigningTag {
    Tx = 0x01,
    RedeemTx = 0x02,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ValidationError {
    InputMissingInUTxO,         // >= Byron
    TxInsEmpty,                 // >= Byron
    TxOutsEmpty,                // >= Byron
    OutputWithoutLovelace,      // == Byron
    UnknownTxSize,              // >= Byron
    UnableToComputeFees,        // >= Byron
    FeesBelowMin,               // >= Byron
    MaxTxSizeExceeded,          // >= Byron
    UnableToProcessWitnesses,   // >= Byron
    MissingWitness,             // >= Byron
    WrongSignature,             // >= Byron
    TTLExceeded,                // >= Shelley
    AlonzoCompatibleNotShelley, // == Shelley
}

pub type ValidationResult = Result<(), ValidationError>;
