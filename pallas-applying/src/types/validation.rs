//! Types for validating transactions in each era.

#[derive(Debug)]
#[non_exhaustive]
pub enum ValidationError {
    TxAndProtParamsDiffer,
    Byron(ByronError),
    ShelleyMA(ShelleyMAError),
    Alonzo(AlonzoError),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ByronError {
    TxInsEmpty,
    TxOutsEmpty,
    InputNotInUTxO,
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
    InputNotInUTxO,
    TTLExceeded,
    AlonzoCompNotShelley,
    UnknownTxSize,
    MaxTxSizeExceeded,
    ValueNotShelley,
    MinLovelaceUnreached,
    PreservationOfValue,
    NegativeValue,
    FeesBelowMin,
    WrongEraOutput,
    AddressDecoding,
    WrongNetworkID,
    MetadataHash,
    MissingVKWitness,
    MissingScriptWitness,
    WrongSignature,
    MintingLacksPolicy,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum AlonzoError {
    UnknownTxSize,
}

pub type ValidationResult = Result<(), ValidationError>;