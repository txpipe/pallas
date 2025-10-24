//! # Errors
//! Errors specific to KES signatures

use crate::kes::common::Depth;
use ed25519_dalek as ed25519;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Clone)]
/// Enum of error associated with KES signatures
pub enum Error {
    /// This error occurs when a base signature (ed25519) is invalid.
    #[error("Ed25519 signature error: {0}")]
    Ed25519Signature(String),
    /// This error occurs when a slice of bytes is converted into a compressed
    /// point format, and it fails.
    #[error("Ed25519 invalid compressed format")]
    Ed25519InvalidCompressedFormat,
    /// Error occurs when the size of the secret key is not the expected.
    #[error("Invalid secret key size: {0}")]
    InvalidSecretKeySize(usize),
    /// Error occurs when the size of the public key is not the expected.
    #[error("Invalid public key size: {0}")]
    InvalidPublicKeySize(usize),
    /// Error occurs when the size of the signature is not the expected.
    #[error("Invalid signature size: {0}")]
    InvalidSignatureSize(usize),
    /// Error occurs when the period associated with a signature is higher than the threshold
    /// allowed by the given `Depth`.
    #[error("Invalid signature count: {0}, Depth: {1}")]
    InvalidSignatureCount(usize, Depth),
    /// Error that occurs when some expected data is found in an only zero slice.
    #[error("Data found in zero area")]
    DataInZeroArea,
    /// This error occurs when a key that cannot be updated (the period has reached the allowed
    /// threshold) tries to be updated.
    #[error("Key cannot be updated more")]
    KeyCannotBeUpdatedMore,
    /// This error occurs when the comparison of two hashes that are expected to be equal fail.
    #[error("Invalid hash comparison")]
    InvalidHashComparison,
}

impl From<ed25519::SignatureError> for Error {
    fn from(sig: ed25519::SignatureError) -> Error {
        Error::Ed25519Signature(format!("{sig}"))
    }
}
