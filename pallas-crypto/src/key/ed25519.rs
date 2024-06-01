//! Ed25519 and Ed25519Extended Asymmetric Keys
//!
//! In this module we have both [`SecretKey`] which is a normal Ed25519
//! asymmetric key and [`SecretKeyExtended`] asymmetric key.
//! They can both be used to generate [`Signature`] and submit valid
//! transactions.
//!
//! However, only the [`SecretKeyExtended`] can be used for HD derivation
//! (using [ed25519_bip32] or otherwise).

use crate::memsec::Scrubbed as _;
use cryptoxide::ed25519::{
    self, EXTENDED_KEY_LENGTH, PRIVATE_KEY_LENGTH, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH,
};
use rand_core::{CryptoRng, RngCore};
use std::{any::type_name, convert::TryFrom, fmt, str::FromStr};
use thiserror::Error;

/// Ed25519 Secret Key
#[derive(Clone)]
pub struct SecretKey([u8; Self::SIZE]);

/// Ed25519 Extended Secret Key
///
/// unlike [`SecretKey`], an extended key can be derived see
/// [`pallas_crypto::derivation`]
#[derive(Clone)]
pub struct SecretKeyExtended([u8; Self::SIZE]);

/// Ed25519 Public Key. Can be used to verify a [`Signature`]. A [`PublicKey`]
/// is associated to a [`SecretKey`]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicKey([u8; Self::SIZE]);

/// Ed25519 Signature. Is created by a [`SecretKey`] and is verified
/// with a [`PublicKey`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signature([u8; Self::SIZE]);

/// Error type used when retrieving a [`PublicKey`] via the [`TryFrom`]
/// trait.
#[derive(Debug, Error)]
pub enum TryFromPublicKeyError {
    #[error("Invalid size, expecting {}", PublicKey::SIZE)]
    InvalidSize,
}

/// Error type used when retrieving a [`Signature`] via the [`TryFrom`]
/// trait.
#[derive(Debug, Error)]
pub enum TryFromSignatureError {
    #[error("Invalid size, expecting {}", Signature::SIZE)]
    InvalidSize,
}

/// Error type used when retrieving a [`SecretKeyExtended`] via
/// [`SecretKeyExtended::from_bytes`] or [`TryFrom`].
///
#[derive(Debug, Error)]
pub enum TryFromSecretKeyExtendedError {
    #[error("Invalid Ed25519 Extended Secret Key format")]
    InvalidBitTweaks,
}

macro_rules! impl_size_zero {
    ($Type:ty, $Size:expr) => {
        impl $Type {
            /// This is the size of the type in bytes.
            pub const SIZE: usize = $Size;

            /// create a zero object. This is not a _"valid"_ one. It is
            /// used to initialize a ready to use data structure in this module.
            #[inline]
            fn zero() -> Self {
                Self([0; Self::SIZE])
            }
        }
    };
}

impl_size_zero!(SecretKey, PRIVATE_KEY_LENGTH);
impl_size_zero!(SecretKeyExtended, EXTENDED_KEY_LENGTH);
impl_size_zero!(PublicKey, PUBLIC_KEY_LENGTH);
impl_size_zero!(Signature, SIGNATURE_LENGTH);

impl SecretKey {
    /// generate a new [`SecretKey`] with the given random number generator
    pub fn new<Rng>(mut rng: Rng) -> Self
    where
        Rng: RngCore + CryptoRng,
    {
        let mut s = Self::zero();
        rng.fill_bytes(&mut s.0);
        s
    }

    /// get the [`PublicKey`] associated to this key
    ///
    /// Unlike the [`SecretKey`], the [`PublicKey`] can be safely
    /// publicly shared. The key can then be used to verify any
    /// [`Signature`] generated with this [`SecretKey`] and the original
    /// message.
    pub fn public_key(&self) -> PublicKey {
        let (mut sk, pk) = ed25519::keypair(&self.0);

        // the `sk` is a private component, scrubbing it reduce the
        // risk of an adversary accessing the memory remains of this
        // value
        sk.scrub();

        PublicKey(pk)
    }

    /// create a [`Signature`] for the given message with this [`SecretKey`].
    ///
    /// The [`Signature`] can then be verified against the associated
    /// [`PublicKey`] and the original message.
    pub fn sign<T>(&self, msg: T) -> Signature
    where
        T: AsRef<[u8]>,
    {
        let (mut sk, _) = ed25519::keypair(&self.0);

        let signature = ed25519::signature(msg.as_ref(), &sk);

        // we don't need this signature component, make sure to scrub the
        // content before releasing the results
        sk.scrub();

        Signature(signature)
    }

    /// convert the [`SecretKey`] into its compressed byte composition
    ///
    /// This function is marked unsafe because we wished to highlight the
    /// importance of keeping the content of the secret key private.
    /// However there are reasons that may be valid to _leak_ the private
    /// key: to encrypt it and store securely (as used in `pallas-wallet`'s
    /// wrapping method for safely storing the private keys).
    ///
    /// This function is on purpose an associated function in order to
    /// force the explicite use of the type name (`SecretKey`) and the
    /// associated function when calling it: `SecretKey::leak_into_bytes(key)`.
    ///
    /// # Safety
    ///
    /// This function is not safe because:
    ///
    /// * using it removes all the security measure we put in place
    ///   to protect your private key: opaque [`Debug`] impl, zeroisation on [`Drop`], ...
    /// * you will need to be careful not to leak the bytes
    ///
    /// # Example
    ///
    /// ```
    /// # use pallas_crypto::key::ed25519::SecretKey;
    /// #
    /// let key: SecretKey = // ...
    /// # [0; SecretKey::SIZE].into() ;
    /// let _: [u8; SecretKey::SIZE] = unsafe { SecretKey::leak_into_bytes(key) };
    /// ```
    ///
    #[inline]
    pub unsafe fn leak_into_bytes(Self(bytes): Self) -> [u8; Self::SIZE] {
        bytes
    }
}

impl SecretKeyExtended {
    /// generate a new [`SecretKeyExtended`] with the given random number
    /// generator
    pub fn new<Rng>(mut rng: Rng) -> Self
    where
        Rng: RngCore + CryptoRng,
    {
        let mut s = Self::zero();
        rng.fill_bytes(&mut s.0);

        s.0[0] &= 0b1111_1000;
        s.0[31] &= 0b0011_1111;
        s.0[31] |= 0b0100_0000;

        debug_assert!(
            s.check_structure(),
            "checking we properly set the bit tweaks for the extended Ed25519"
        );

        s
    }

    #[inline]
    #[allow(clippy::verbose_bit_mask)]
    fn check_structure(&self) -> bool {
        (self.0[0] & 0b0000_0111) == 0
            && (self.0[31] & 0b0100_0000) == 0b0100_0000
            && (self.0[31] & 0b1000_0000) == 0
    }

    /// Retrieve a [`SecretKeyExtended`] from the given `bytes`` array.
    ///
    /// # error
    ///
    /// This function will check that the given bytes are valid for
    /// an Ed25519 Extended Secret key. I.e. it will check that the
    /// proper bits have been zeroed.
    ///
    /// # Example
    ///
    /// ```
    /// # use pallas_crypto::key::ed25519::{SecretKeyExtended, TryFromSecretKeyExtendedError};
    /// #
    /// # fn test() -> Result<(), TryFromSecretKeyExtendedError> {
    /// let bytes = // ...
    /// # [0; 64] ;
    /// let key = SecretKeyExtended::from_bytes(bytes)?;
    /// # let _ = key; Ok(()) }
    /// # assert!(matches!(test(), Err(TryFromSecretKeyExtendedError::InvalidBitTweaks)));
    /// ```
    ///
    pub fn from_bytes(bytes: [u8; Self::SIZE]) -> Result<Self, TryFromSecretKeyExtendedError> {
        let candidate = Self(bytes);
        if candidate.check_structure() {
            Ok(candidate)
        } else {
            Err(TryFromSecretKeyExtendedError::InvalidBitTweaks)
        }
    }

    /// Retrieve a [`SecretKeyExtended`] from the given bytes
    ///
    /// **You should prefer [`SecretKeyExtended::from_bytes`] instead
    /// as this function does not check that the bytes are correct
    /// for Ed25519 Extended**
    ///
    /// # Safety
    ///
    /// This function creates a [`SecretKeyExtended`] without checking
    /// the validity of the bytes (the bits tweaked or not).
    ///
    /// It will not panic but using the created key may result to undefined
    /// behavior or may generate [`Signature`] that are not cryptographically
    /// secure.
    ///
    /// # Example
    ///
    /// ```
    /// # use pallas_crypto::key::ed25519::SecretKeyExtended;
    /// #
    /// let bytes = // ...
    /// # [0; 64] ;
    /// let key = unsafe { SecretKeyExtended::from_bytes_unchecked(bytes) };
    /// # let _ = key;
    /// ```
    ///
    pub unsafe fn from_bytes_unchecked(bytes: [u8; Self::SIZE]) -> Self {
        Self(bytes)
    }

    /// get the [`PublicKey`] associated to this key
    ///
    /// Unlike the [`SecretKeyExtended`], the [`PublicKey`] can be safely
    /// publicly shared. The key can then be used to verify any
    /// [`Signature`] generated with this [`SecretKeyExtended`] and the original
    /// message.
    pub fn public_key(&self) -> PublicKey {
        let pk = ed25519::extended_to_public(&self.0);

        PublicKey::from(pk)
    }

    /// create a `Signature` for the given message with this `SecretKey`.
    ///
    /// The `Signature` can then be verified against the associated `PublicKey`
    /// and the original message.
    pub fn sign<T: AsRef<[u8]>>(&self, msg: T) -> Signature {
        let signature = ed25519::signature_extended(msg.as_ref(), &self.0);

        Signature::from(signature)
    }

    /// convert the [`SecretKeyExtended`] into its compressed byte composition
    ///
    /// This function is marked unsafe because we wished to highlight the
    /// importance of keeping the content of the secret key private.
    /// However there are reasons that may be valid to _leak_ the private
    /// key: to encrypt it and store securely (as used in `pallas-wallet`'s
    /// wrapping method for safely storing the private keys).
    ///
    /// This function is on purpose an associated function in order to
    /// force the explicite use of the type name (`SecretKeyExtended`) and the
    /// associated function when calling it: `SecretKeyExtended::leak_into_bytes(key)`.
    ///
    /// # Safety
    ///
    /// This function is not safe because:
    ///
    /// * using it removes all the security measure we put in place
    ///   to protect your private key: opaque [`Debug`] impl, zeroisation on [`Drop`], ...
    /// * you will need to be careful not to leak the bytes
    ///
    /// # Example
    ///
    /// ```
    /// # use pallas_crypto::key::ed25519::SecretKeyExtended;
    /// #
    /// let key: SecretKeyExtended = // ...
    /// # unsafe { SecretKeyExtended::from_bytes_unchecked([0; SecretKeyExtended::SIZE]) };
    /// let _: [u8; SecretKeyExtended::SIZE] = unsafe { SecretKeyExtended::leak_into_bytes(key) };
    /// ```
    ///
    #[inline]
    pub unsafe fn leak_into_bytes(Self(bytes): Self) -> [u8; Self::SIZE] {
        bytes
    }
}

impl PublicKey {
    /// verify the cryptographic [`Signature`] against the `message` and the
    /// [`PublicKey`] `self`.
    #[inline]
    pub fn verify<T>(&self, message: T, signature: &Signature) -> bool
    where
        T: AsRef<[u8]>,
    {
        ed25519::verify(message.as_ref(), &self.0, &signature.0)
    }
}

/* Drop ******************************************************************** */

impl Drop for SecretKey {
    fn drop(&mut self) {
        self.0.scrub()
    }
}

impl Drop for SecretKeyExtended {
    fn drop(&mut self) {
        self.0.scrub()
    }
}

/* Format ****************************************************************** */

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.as_ref()))
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.as_ref()))
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Signature<Ed25519>")
            .field(&hex::encode(self.as_ref()))
            .finish()
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PublicKey<Ed25519>")
            .field(&hex::encode(self.as_ref()))
            .finish()
    }
}

macro_rules! impl_secret_fmt {
    ($Type:ty) => {
        /// conveniently provide a proper implementation to debug for the
        /// SecretKey types when only *testing* the library
        #[cfg(test)]
        impl fmt::Debug for $Type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(&format!(
                    "SecretKey<{typename}>",
                    typename = type_name::<Self>()
                ))
                .field(&hex::encode(&self.0))
                .finish()
            }
        }

        /// conveniently provide an incomplete implementation of Debug for the
        /// SecretKey.
        #[cfg(not(test))]
        impl fmt::Debug for $Type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(&format!(
                    "SecretKey<{typename}>",
                    typename = type_name::<Self>()
                ))
                .finish_non_exhaustive()
            }
        }
    };
}

impl_secret_fmt!(SecretKey);
impl_secret_fmt!(SecretKeyExtended);

/* AsRef ******************************************************************* */

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/* Conversion ************************************************************** */

impl<'a> From<&'a Signature> for String {
    fn from(s: &'a Signature) -> Self {
        s.to_string()
    }
}

impl From<Signature> for String {
    fn from(s: Signature) -> Self {
        s.to_string()
    }
}

impl From<[u8; Self::SIZE]> for PublicKey {
    fn from(bytes: [u8; Self::SIZE]) -> Self {
        Self(bytes)
    }
}

impl From<PublicKey> for [u8; PublicKey::SIZE] {
    fn from(pk: PublicKey) -> Self {
        pk.0
    }
}

impl From<[u8; Self::SIZE]> for Signature {
    fn from(bytes: [u8; Self::SIZE]) -> Self {
        Self(bytes)
    }
}

impl From<[u8; Self::SIZE]> for SecretKey {
    fn from(bytes: [u8; Self::SIZE]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<[u8; Self::SIZE]> for SecretKeyExtended {
    type Error = TryFromSecretKeyExtendedError;
    fn try_from(bytes: [u8; Self::SIZE]) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl<'a> TryFrom<&'a [u8]> for PublicKey {
    type Error = TryFromPublicKeyError;
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() != Self::SIZE {
            Err(Self::Error::InvalidSize)
        } else {
            let mut s = Self::zero();
            s.0.copy_from_slice(value);
            Ok(s)
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for Signature {
    type Error = TryFromSignatureError;
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() != Self::SIZE {
            Err(Self::Error::InvalidSize)
        } else {
            let mut s = Self::zero();
            s.0.copy_from_slice(value);
            Ok(s)
        }
    }
}

impl FromStr for PublicKey {
    type Err = hex::FromHexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut r = Self::zero();
        hex::decode_to_slice(s, &mut r.0)?;
        Ok(r)
    }
}

impl FromStr for Signature {
    type Err = hex::FromHexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut r = Self::zero();
        hex::decode_to_slice(s, &mut r.0)?;
        Ok(r)
    }
}

impl<'a> TryFrom<&'a str> for Signature {
    type Error = <Self as FromStr>::Err;
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;

    impl Arbitrary for SecretKey {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut s = Self::zero();
            s.0.iter_mut().for_each(|byte| {
                *byte = u8::arbitrary(g);
            });
            s
        }
    }

    impl Arbitrary for SecretKeyExtended {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut s = Self::zero();
            s.0.iter_mut().for_each(|byte| {
                *byte = u8::arbitrary(g);
            });

            s.0[0] &= 0b1111_1000;
            s.0[31] &= 0b0011_1111;
            s.0[31] |= 0b0100_0000;

            s
        }
    }

    impl Arbitrary for PublicKey {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut s = Self::zero();
            s.0.iter_mut().for_each(|byte| {
                *byte = u8::arbitrary(g);
            });
            s
        }
    }

    impl Arbitrary for Signature {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut s = Self::zero();
            s.0.iter_mut().for_each(|byte| {
                *byte = u8::arbitrary(g);
            });
            s
        }
    }

    #[quickcheck]
    fn signing_verify_works(signing_key: SecretKey, message: Vec<u8>) -> bool {
        let public_key = signing_key.public_key();
        let signature = signing_key.sign(&message);

        public_key.verify(message, &signature)
    }

    #[quickcheck]
    fn signing_verify_works_extended(signing_key: SecretKeyExtended, message: Vec<u8>) -> bool {
        let public_key = signing_key.public_key();
        let signature = signing_key.sign(&message);

        public_key.verify(message, &signature)
    }

    #[quickcheck]
    fn verify_random_signature_does_not_work(
        public_key: PublicKey,
        signature: Signature,
        message: Vec<u8>,
    ) -> bool {
        // NOTE: this test may fail but it is impossible to see this happening in normal
        // condition. We are generating 32 random bytes of public key and
        // 64 random bytes of signature with an randomly generated message
        // of a random number of bytes in. If the message were empty, the
        // probability to have a signature that matches the verify key

        // would still be 1 out of 2^96.
        //
        // if this test fails and it is not a bug, go buy a lottery ticket.
        !public_key.verify(message, &signature)
    }

    #[quickcheck]
    fn public_key_try_from_correct_size(public_key: PublicKey) -> TestResult {
        match PublicKey::try_from(public_key.as_ref()) {
            Ok(_) => TestResult::passed(),
            Err(TryFromPublicKeyError::InvalidSize) => {
                TestResult::error("was expecting the test to pass")
            }
        }
    }

    #[quickcheck]
    fn public_key_try_from_incorrect_size(bytes: Vec<u8>) -> TestResult {
        if bytes.len() == PublicKey::SIZE {
            return TestResult::discard();
        }
        match PublicKey::try_from(bytes.as_slice()) {
            Ok(_) => TestResult::error(
                "Expecting to fail with invalid size instead of having a valid value",
            ),
            Err(TryFromPublicKeyError::InvalidSize) => TestResult::passed(),
        }
    }

    #[quickcheck]
    fn signature_try_from_correct_size(signature: Signature) -> TestResult {
        match Signature::try_from(signature.as_ref()) {
            Ok(_) => TestResult::passed(),
            Err(TryFromSignatureError::InvalidSize) => {
                TestResult::error("was expecting the test to pass")
            }
        }
    }

    #[quickcheck]
    fn signature_try_from_incorrect_size(bytes: Vec<u8>) -> TestResult {
        if bytes.len() == Signature::SIZE {
            return TestResult::discard();
        }
        match Signature::try_from(bytes.as_slice()) {
            Ok(_) => TestResult::error(
                "Expecting to fail with invalid size instead of having a valid value",
            ),
            Err(TryFromSignatureError::InvalidSize) => TestResult::passed(),
        }
    }

    #[quickcheck]
    fn public_key_from_str(public_key: PublicKey) -> TestResult {
        let s = public_key.to_string();

        match s.parse::<PublicKey>() {
            Ok(decoded) => {
                if decoded == public_key {
                    TestResult::passed()
                } else {
                    TestResult::error("the decoded key is not equal")
                }
            }
            Err(error) => TestResult::error(error.to_string()),
        }
    }

    #[quickcheck]
    fn signature_from_str(signature: Signature) -> TestResult {
        let s = signature.to_string();

        match s.parse::<Signature>() {
            Ok(decoded) => {
                if decoded == signature {
                    TestResult::passed()
                } else {
                    TestResult::error("the decoded signature is not equal")
                }
            }
            Err(error) => TestResult::error(error.to_string()),
        }
    }
}
