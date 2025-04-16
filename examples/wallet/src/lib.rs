use pallas_crypto::key::ed25519::{
    PublicKey, SecretKey, SecretKeyExtended, Signature, TryFromSecretKeyExtendedError,
};
use thiserror::Error;

pub mod hd;
pub mod wrapper;

#[derive(Error, Debug)]
pub enum Error {
    /// Private key wrapper data of unexpected length
    #[error("Wrapped private key data invalid length")]
    WrapperDataInvalidSize,
    /// Failed to decrypt private key wrapper data
    #[error("Failed to decrypt private key wrapper data")]
    WrapperDataFailedToDecrypt,
    /// Unexpected bech32 HRP prefix
    #[error("Unexpected bech32 HRP prefix")]
    InvalidBech32Hrp,
    /// Unable to decode bech32 string
    #[error("Unable to decode bech32: {0}")]
    InvalidBech32(bech32::Error),
    /// Decoded bech32 data of unexpected length
    #[error("Decoded bech32 data of unexpected length")]
    UnexpectedBech32Length,
    /// Error relating to ed25519-bip32 private key
    #[error("Error relating to ed25519-bip32 private key: {0}")]
    Xprv(ed25519_bip32::PrivateKeyError),
    /// Error relating to bip39 mnemonic
    #[error("Error relating to bip39 mnemonic: {0}")]
    Mnemonic(bip39::Error),
    /// Error when attempting to derive ed25519-bip32 key
    #[error("Error when attempting to derive ed25519-bip32 key: {0}")]
    DerivationError(ed25519_bip32::DerivationError),
    /// Error that may occurs when trying to decrypt a private key
    /// which is not valid.
    #[error("Invalid Ed25519 Extended Secret Key: {0}")]
    InvalidSecretKeyExtended(#[from] TryFromSecretKeyExtendedError),
}

/// A standard or extended Ed25519 secret key
pub enum PrivateKey {
    Normal(SecretKey),
    Extended(SecretKeyExtended),
}

impl PrivateKey {
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            Self::Normal(_) => SecretKey::SIZE,
            Self::Extended(_) => SecretKeyExtended::SIZE,
        }
    }

    pub fn public_key(&self) -> PublicKey {
        match self {
            Self::Normal(x) => x.public_key(),
            Self::Extended(x) => x.public_key(),
        }
    }

    pub fn sign<T>(&self, msg: T) -> Signature
    where
        T: AsRef<[u8]>,
    {
        match self {
            Self::Normal(x) => x.sign(msg),
            Self::Extended(x) => x.sign(msg),
        }
    }

    pub(crate) fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Normal(x) => {
                let bytes: [u8; SecretKey::SIZE] = unsafe { SecretKey::leak_into_bytes(x.clone()) };
                bytes.to_vec()
            }
            Self::Extended(x) => {
                let bytes: [u8; SecretKeyExtended::SIZE] =
                    unsafe { SecretKeyExtended::leak_into_bytes(x.clone()) };
                bytes.to_vec()
            }
        }
    }
}

impl From<SecretKey> for PrivateKey {
    fn from(key: SecretKey) -> Self {
        PrivateKey::Normal(key)
    }
}

impl From<SecretKeyExtended> for PrivateKey {
    fn from(key: SecretKeyExtended) -> Self {
        PrivateKey::Extended(key)
    }
}
