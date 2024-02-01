//! Types for validating transactions in each era.

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ValidationError {
    TxAndProtParamsDiffer,
    Byron(ByronError),
    ShelleyMA(ShelleyMAError),
    Alonzo(AlonzoError),
    Babbage(BabbageError),
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AlonzoError {
    UnknownTxSize,
    TxInsEmpty,
    InputNotInUTxO,
    CollateralNotInUTxO,
    BlockExceedsValInt,
    BlockPrecedesValInt,
    ValIntUpperBoundMissing,
    FeeBelowMin,
    CollateralMissing,
    TooManyCollaterals,
    CollateralNotVKeyLocked,
    AddressDecoding,
    CollateralMinLovelace,
    NonLovelaceCollateral,
    NegativeValue,
    PreservationOfValue,
    MinLovelaceUnreached,
    MaxValSizeExceeded,
    OutputWrongNetworkID,
    TxWrongNetworkID,
    RedeemerMissing,
    TxExUnitsExceeded,
    MaxTxSizeExceeded,
    VKWitnessMissing,
    VKWrongSignature,
    ReqSignerMissing,
    ReqSignerWrongSig,
    ScriptWitnessMissing,
    MintingLacksPolicy,
    InputDecoding,
    UnneededNativeScript,
    UnneededPlutusScript,
    UnneededRedeemer,
    DatumMissing,
    UnneededDatum,
    MetadataHash,
    ScriptIntegrityHash,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BabbageError {
    UnknownTxSize,
    TxInsEmpty,
    InputNotInUTxO,
    CollateralNotInUTxO,
    RefInputNotInUTxO,
    BlockPrecedesValInt,
    BlockExceedsValInt,
    FeeBelowMin,
    CollateralMissing,
    TooManyCollaterals,
    InputDecoding,
    CollateralNotVKeyLocked,
    CollateralMinLovelace,
    NonLovelaceCollateral,
    CollateralWrongAssets,
    NegativeValue,
    CollateralAnnotation,
    PreservationOfValue,
    MinLovelaceUnreached,
    MaxValSizeExceeded,
    AddressDecoding,
    OutputWrongNetworkID,
    TxWrongNetworkID,
    TxExUnitsExceeded,
    RedeemerMissing,
    MaxTxSizeExceeded,
}

pub type ValidationResult = Result<(), ValidationError>;
