//! Traits that define a KES signature instance
use crate::kes::common::PublicKey;
use crate::kes::errors::Error;

/// Trait that defined a Kes secret key
pub trait KesSk<'a>: Sized {
    /// Type of the associated signature
    type Sig;
    /// Size of SK
    const SIZE: usize;

    /// Key generation
    fn keygen(key_buffer: &'a mut [u8], seed: &'a mut [u8]) -> (Self, PublicKey);

    /// KES signature, using `self`.
    fn sign(&self, m: &[u8]) -> Self::Sig;

    /// Update key by taking a mutable reference to `self`
    fn update(&mut self) -> Result<(), Error>;

    /// Return the current period of the secret key
    fn get_period(&self) -> u32;

    /// Convert the slice of bytes into `Self`.
    ///
    /// # Errors
    /// The function fails if
    /// * `bytes.len()` is not of the expected size
    fn from_bytes(bytes: &'a mut [u8]) -> Result<Self, Error>;

    /// Convert `Self` into it's byte representation. In particular, the encoding returns
    /// the following array of size `Self::SIZE + 4`:
    /// ( sk_{-1} || seed || self.lhs_pk || self.rhs_pk || period )
    /// where `sk_{-1}` is the secret secret key of lower depth.
    /// Note that the period is only included in the last layer.
    fn as_bytes(&self) -> &[u8];
}

/// Trait that defines a KES signature
///
/// # Example
/// ```
/// use pallas_crypto::kes::summed_kes::Sum6Kes;
/// use pallas_crypto::kes::traits::{KesSig, KesSk};
/// // The function caller needs to allocate memory for the secret key
/// let mut key_buffer = [0u8; Sum6Kes::SIZE + 4];
/// let mut seed = [0u8; 32];
/// let (mut skey, pkey) = Sum6Kes::keygen(&mut key_buffer, &mut seed);
/// let dummy_message = b"tilin";
/// let sigma = skey.sign(dummy_message);
///
/// assert!(sigma.verify(0, &pkey, dummy_message).is_ok());
///
/// // Key can be updated 63 times
/// for _ in 0..63 {
///     assert!(skey.update().is_ok());
/// }
/// ```
pub trait KesSig: Sized {
    /// Verify the signature
    fn verify(&self, period: u32, pk: &PublicKey, m: &[u8]) -> Result<(), Error>;
}

/// Trait that defined a CompactKES signature. Instead of recursively verifying, we simply
/// verify once (equality with the root), and else we recompute the root of the subtree.
/// When we reach the leaf, we also verify the ed25519 signature.
pub trait KesCompactSig: Sized {
    /// Verify the root equality
    fn verify(&self, period: u32, pk: &PublicKey, m: &[u8]) -> Result<(), Error> {
        let pk_subtree = self.recompute(period, m)?;
        if pk == &pk_subtree {
            return Ok(());
        }
        Err(Error::InvalidHashComparison)
    }
    /// Recompute the root of the subtree, and verify ed25519 if on leaf
    fn recompute(&self, period: u32, m: &[u8]) -> Result<PublicKey, Error>;
}
