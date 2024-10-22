mod babbage;
mod scriptdata;
mod transaction;

pub use babbage::BuildBabbage;
pub use transaction::model::{
    BuiltTransaction, ExUnits, Input, Output, ScriptKind, StagingTransaction,
};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TxBuilderError {
    /// Provided bytes could not be decoded into a script
    #[error("Transaction has no inputs")]
    MalformedScript,
    /// Provided bytes could not be decoded into a datum
    #[error("Could not decode datum bytes")]
    MalformedDatum,
    /// Provided datum hash was not 32 bytes in length
    #[error("Invalid bytes length for datum hash")]
    MalformedDatumHash,
    /// Input, policy, etc pointed to by a redeemer was not found in the
    /// transaction
    #[error("Input/policy pointed to by redeemer not found in tx")]
    RedeemerTargetMissing,
    /// Provided network ID is invalid (must be 0 or 1)
    #[error("Invalid network ID")]
    InvalidNetworkId,
    /// Transaction bytes in built transaction object could not be decoded
    #[error("Corrupted transaction bytes in built transaction")]
    CorruptedTxBytes,
    /// Public key generated from private key was of unexpected length
    #[error("Public key for private key is malformed")]
    MalformedKey,
    /// Asset name is too long, it must be 32 bytes or less
    #[error("Asset name must be 32 bytes or less")]
    AssetNameTooLong,
}
