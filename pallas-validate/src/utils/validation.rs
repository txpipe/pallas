//! Types for validating transactions in each era.
use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum ValidationError {
    #[error("transaction and protocol parameters differ")]
    TxAndProtParamsDiffer,

    #[error("byron doesn't need account state")]
    PParamsByronDoesntNeedAccountState,

    #[error("missing account state")]
    EnvMissingAccountState,

    #[error("unknown protocol parameters")]
    UnknownProtParams,

    #[error("{0}")]
    Byron(ByronError),

    #[error("{0}")]
    ShelleyMA(ShelleyMAError),

    #[error("{0}")]
    Alonzo(AlonzoError),

    #[error("{0}")]
    PostAlonzo(PostAlonzoError),
}

#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum ByronError {
    #[error("transaction has no inputs")]
    TxInsEmpty,

    #[error("transaction has no outputs")]
    TxOutsEmpty,

    #[error("some transaction input is not present in the UTxO set")]
    InputNotInUTxO,

    #[error("transaction output has no lovelace")]
    OutputWithoutLovelace,

    #[error("transaction size could not be determined")]
    UnknownTxSize,

    #[error("transaction fee cannot be computed")]
    UnableToComputeFees,

    #[error("transaction fee is below the minimum fee required")]
    FeesBelowMin,

    #[error("transaction size exceeds the maximum allowed")]
    MaxTxSizeExceeded,

    #[error("unable to process witnesses from the transaction")]
    UnableToProcessWitness,

    #[error("transaction witness is missing")]
    MissingWitness,

    #[error("transaction has the wrong signature")]
    WrongSignature,
}

#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum ShelleyMAError {
    #[error("transaction has no inputs")]
    TxInsEmpty,

    #[error("some transaction input is not present in the UTxO set")]
    InputNotInUTxO,

    #[error("transaction exceeds the TTL range")]
    TTLExceeded,

    #[error("transaction composition is not compatible with shelley")]
    AlonzoCompNotShelley,

    #[error("transaction size could not be determined")]
    UnknownTxSize,

    #[error("transaction size exceeds the maximum allowed")]
    MaxTxSizeExceeded,

    #[error("some value in the transaction is not shelley-compatible")]
    ValueNotShelley,

    #[error("minimum lovelace requirement was not met")]
    MinLovelaceUnreached,

    #[error("transaction values are not preserved correctly")]
    PreservationOfValue,

    #[error("transaction contains a negative value")]
    NegativeValue,

    #[error("transaction fee is below the minimum fee required")]
    FeesBelowMin,

    #[error("transaction output is from a different era")]
    WrongEraOutput,

    #[error("failed to decode the address")]
    AddressDecoding,

    #[error("transaction has the wrong network ID")]
    WrongNetworkID,

    #[error("metadata hash is invalid")]
    MetadataHash,

    #[error("vkey witness is missing")]
    MissingVKWitness,

    #[error("script witness is missing")]
    MissingScriptWitness,

    #[error("transaction has the wrong signature")]
    WrongSignature,

    #[error("minting lacks the required policy")]
    MintingLacksPolicy,

    #[error("key is already registered")]
    KeyAlreadyRegistered,

    #[error("key is not yet registered")]
    KeyNotRegistered,

    #[error("pointer is already in use")]
    PointerInUse,

    #[error("rewards are not null")]
    RewardsNotNull,

    #[error("pool is already registered")]
    PoolAlreadyRegistered,

    #[error("pool is not yet registered")]
    PoolNotRegistered,

    #[error("pool cost is below the minimum")]
    PoolCostBelowMin,

    #[error("transaction has duplicate genesis delegates")]
    DuplicateGenesisDelegate,

    #[error("transaction has duplicate genesis VRF")]
    DuplicateGenesisVRF,

    #[error("genesis key is not in the mapping")]
    GenesisKeyNotInMapping,

    #[error("insufficient funds for instantaneous rewards")]
    InsufficientForInstantaneousRewards,

    #[error("MIR certificate is too late in the epoch")]
    MIRCertificateTooLateinEpoch,

    #[error("script is denied")]
    ScriptDenial,
}

#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum AlonzoError {
    #[error("transaction size could not be determined")]
    UnknownTxSize,

    #[error("transaction has no inputs")]
    TxInsEmpty,

    #[error("some transaction input is not present in the UTxO set")]
    InputNotInUTxO,

    #[error("collateral is not present in the UTxO set")]
    CollateralNotInUTxO,

    #[error("block precedes the validity interval")]
    BlockExceedsValInt,

    #[error("block exceeds the validity interval")]
    BlockPrecedesValInt,

    #[error("upper bound of the validity interval is missing")]
    ValIntUpperBoundMissing,

    #[error("transaction fee is below the minimum fee required")]
    FeeBelowMin,

    #[error("collateral input is missing from the transaction")]
    CollateralMissing,

    #[error("transaction contains too many collateral inputs")]
    TooManyCollaterals,

    #[error("the collateral input is not VKey locked")]
    CollateralNotVKeyLocked,

    #[error("failed to decode the address")]
    AddressDecoding,

    #[error("collateral input does not meet the minimum lovelace required")]
    CollateralMinLovelace,

    #[error("collateral input contains non-lovelace assets")]
    NonLovelaceCollateral,

    #[error("transaction contains a negative value")]
    NegativeValue,

    #[error("transaction values are not preserved correctly")]
    PreservationOfValue,

    #[error("minimum lovelace requirement was not met")]
    MinLovelaceUnreached,

    #[error("the maximum value size has been exceeded")]
    MaxValSizeExceeded,

    #[error("transaction output has the wrong network ID")]
    OutputWrongNetworkID,

    #[error("transaction has the wrong network ID")]
    TxWrongNetworkID,

    #[error("required redeemer is missing")]
    RedeemerMissing,

    #[error("the transaction's execution units exceed the maximum allowed")]
    TxExUnitsExceeded,

    #[error("transaction size exceeds the maximum allowed")]
    MaxTxSizeExceeded,

    #[error("vkey witness is missing")]
    VKWitnessMissing,

    #[error("vkey witness has the wrong signature")]
    VKWrongSignature,

    #[error("required signer is missing")]
    ReqSignerMissing,

    #[error("required signer has the wrong signature")]
    ReqSignerWrongSig,

    #[error("script witness is missing")]
    ScriptWitnessMissing,

    #[error("minting lacks the required policy")]
    MintingLacksPolicy,

    #[error("failed to decode the input")]
    InputDecoding,

    #[error("an unnecessary native script is present")]
    UnneededNativeScript,

    #[error("an unnecessary Plutus V1 script is present")]
    UnneededPlutusScript,

    #[error("an unnecessary redeemer is present")]
    UnneededRedeemer,

    #[error("required datum is missing")]
    DatumMissing,

    #[error("an unnecessary datum is present")]
    UnneededDatum,

    #[error("metadata hash is invalid")]
    MetadataHash,

    #[error("invalid script integrity hash")]
    ScriptIntegrityHash,
}

#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum PostAlonzoError {
    #[error("transaction size could not be determined")]
    UnknownTxSize,

    #[error("transaction has no inputs")]
    TxInsEmpty,

    #[error("some transaction input is not present in the UTxO set")]
    InputNotInUTxO,

    #[error("collateral is not present in the UTxO set")]
    CollateralNotInUTxO,

    #[error("reference input is not present in the UTxO set")]
    ReferenceInputNotInUTxO,

    #[error("block precedes the validity interval")]
    BlockPrecedesValInt,

    #[error("block exceeds the validity interval")]
    BlockExceedsValInt,

    #[error("transaction fee is below the minimum fee required")]
    FeeBelowMin,

    #[error("collateral input is missing from the transaction")]
    CollateralMissing,

    #[error("transaction contains too many collateral inputs")]
    TooManyCollaterals,

    #[error("failed to decode the input")]
    InputDecoding,

    #[error("the collateral input is not VKey locked")]
    CollateralNotVKeyLocked,

    #[error("collateral input does not meet the minimum lovelace required")]
    CollateralMinLovelace,

    #[error("collateral input contains non-lovelace assets")]
    NonLovelaceCollateral,

    #[error("collateral input contains incorrect assets")]
    CollateralWrongAssets,

    #[error("transaction contains a negative value")]
    NegativeValue,

    #[error("paid collateral does not match the annotated collateral")]
    CollateralAnnotation,

    #[error("transaction values are not preserved correctly")]
    PreservationOfValue,

    #[error("minimum lovelace requirement was not met")]
    MinLovelaceUnreached,

    #[error("the maximum value size has been exceeded")]
    MaxValSizeExceeded,

    #[error("failed to decode the address")]
    AddressDecoding,

    #[error("transaction output has the wrong network ID")]
    OutputWrongNetworkID,

    #[error("transaction has the wrong network ID")]
    TxWrongNetworkID,

    #[error("the transaction's execution units exceed the maximum allowed")]
    TxExUnitsExceeded,

    #[error("required redeemer is missing")]
    RedeemerMissing,

    #[error("an unnecessary redeemer is present")]
    UnneededRedeemer,

    #[error("transaction size exceeds the maximum allowed")]
    MaxTxSizeExceeded,

    #[error("minting lacks the required policy")]
    MintingLacksPolicy,

    #[error("metadata hash is invalid")]
    MetadataHash,

    #[error("required datum is missing")]
    DatumMissing,

    #[error("an unnecessary datum is present")]
    UnneededDatum,

    #[error("script witness is missing")]
    ScriptWitnessMissing,

    #[error("an unnecessary native script is present")]
    UnneededNativeScript,

    #[error("an unnecessary Plutus V1 script is present")]
    UnneededPlutusV1Script,

    #[error("an unnecessary Plutus V2 script is present")]
    UnneededPlutusV2Script,

    #[error("an unnecessary Plutus V3 script is present")]
    UnneededPlutusV3Script,

    #[error("required signer is missing")]
    ReqSignerMissing,

    #[error("required signer has the wrong signature")]
    ReqSignerWrongSig,

    #[error("vkey witness is missing")]
    VKWitnessMissing,

    #[error("vkey witness has the wrong signature")]
    VKWrongSignature,

    #[error("transaction contains an unsupported plutus language")]
    UnsupportedPlutusLanguage,

    #[error("invalid script integrity hash")]
    ScriptIntegrityHash,

    #[error("transaction data does not satisfy business/CDDL invariant")]
    BrokenBusinessInvariant,
}

pub type ValidationResult = Result<(), ValidationError>;
