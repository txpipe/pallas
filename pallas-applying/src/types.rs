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
pub struct ShelleyProtParams {
    pub max_tx_size: u64,
    pub min_lovelace: u64,
}

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
    pub network_id: u8,
}

#[non_exhaustive]
pub enum SigningTag {
    Tx = 0x01,
    RedeemTx = 0x02,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ValidationError {
    Byron(ByronError),
    Shelley(ShelleyMAError),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ByronError {
    TxInsEmpty,
    TxOutsEmpty,
    InputMissingInUTxO,
    OutputWithoutLovelace,
    UnknownTxSize,
    UnableToComputeFees,
    FeesBelowMin,
    MaxTxSizeExceeded,
    UnableToProcessWitness,
    MissingWitness,
    WrongSignature,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ShelleyMAError {
    TxInsEmpty,
    InputMissingInUTxO,
    TTLExceeded,
    AlonzoCompNotShelley,
    UnknownTxSize,
    MaxTxSizeExceeded,
    ValueNotShelley,
    MinLovelaceUnreached,
    WrongEraOutput,
    AddressDecoding,
    WrongNetworkID,
}

pub type ValidationResult = Result<(), ValidationError>;
