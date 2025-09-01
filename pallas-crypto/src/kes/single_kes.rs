//! Implementation of the base signature used for KES. This is a standard signature
//! mechanism which is considered a KES signature scheme with a single period. In this
//! case, the single instance is ed25519.
use std::convert::TryInto;

use crate::kes::common::{PublicKey, Seed};
use crate::kes::errors::Error;
use crate::kes::traits::{KesCompactSig, KesSig, KesSk};
use ed25519_dalek::{
    Signature as EdSignature, Signer, SigningKey as EdSigningKey, VerifyingKey as EdPublicKey,
    SIGNATURE_LENGTH,
};
pub use ed25519_dalek::{PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug)]
/// Single KES instance, which is a wrapper over ed25519.
pub struct Sum0Kes<'a>(pub(crate) &'a mut [u8]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde_as]
/// Single KES Signature instance, which is a wrapper over ed25519.
pub struct Sum0KesSig(#[serde_as(as = "Bytes")] pub(crate) EdSignature);

impl Drop for Sum0Kes<'_> {
    fn drop(&mut self) {
        self.0.copy_from_slice(&[0u8; Self::SIZE + 4])
    }
}

impl<'a> KesSk<'a> for Sum0Kes<'a> {
    type Sig = Sum0KesSig;
    const SIZE: usize = SECRET_KEY_LENGTH;

    fn keygen(key_buffer: &'a mut [u8], master_seed: &mut [u8]) -> (Self, PublicKey) {
        assert_eq!(key_buffer.len(), Self::SIZE + 4);
        assert_eq!(master_seed.len(), 32);

        let secret = EdSigningKey::from_bytes(
            &master_seed
                .try_into()
                .expect("Seed is defined with 32 bytes, so it won't fail."),
        );

        let public = (&secret).into();
        // We copy the secret key to the key buffer and we drop the secret key (which zeros de data)
        key_buffer[..32].copy_from_slice(&secret.to_bytes());
        drop(secret);
        master_seed.copy_from_slice(&[0u8; 32]);
        (
            Sum0Kes(key_buffer),
            PublicKey::from_ed25519_verifyingkey(&public),
        )
    }

    fn update(&mut self) -> Result<(), Error> {
        Err(Error::KeyCannotBeUpdatedMore)
    }

    fn sign(&self, m: &[u8]) -> Sum0KesSig {
        let secret = EdSigningKey::from_bytes(
            (&*self.0)
                .try_into()
                .expect("Seed is defined with 32 bytes, so it won't fail."),
        );
        Sum0KesSig(secret.sign(m))
    }
    fn from_bytes(bytes: &'a mut [u8]) -> Result<Self, Error> {
        if bytes.len() != Self::SIZE + 4 {
            // We need to account for the seed
            return Err(Error::InvalidSecretKeySize(bytes.len()));
        }

        Ok(Self(bytes))
    }

    fn as_bytes(&self) -> &[u8] {
        self.0
    }

    fn get_period(&self) -> u32 {
        0
    }
}

impl<'a> Sum0Kes<'a> {
    pub(crate) fn update_slice(_: &mut [u8], _: u32) -> Result<(), Error> {
        Err(Error::KeyCannotBeUpdatedMore)
    }

    pub(crate) fn keygen_slice(in_slice: &mut [u8], opt_seed: Option<&mut [u8]>) -> PublicKey {
        let secret = if let Some(seed) = opt_seed {
            assert_eq!(in_slice.len(), Self::SIZE, "Input size is incorrect.");

            let sk =
                EdSigningKey::from_bytes(&seed.try_into().expect("Size of the seed is incorrect."));

            seed.copy_from_slice(&[0u8; 32]);
            sk
        } else {
            assert_eq!(
                in_slice.len(),
                Self::SIZE + Seed::SIZE,
                "Input size is incorrect."
            );

            let sk = EdSigningKey::from_bytes(
                &in_slice[Self::SIZE..]
                    .try_into()
                    .expect("Size of the seed is incorrect."),
            );

            in_slice[Self::SIZE..].copy_from_slice(&[0u8; 32]);
            sk
        };

        let public = (&secret).into();

        // We need to make this copies unfortunately by how the
        // underlying library behaves. Would be great to have a
        // EdPubKey from seed function.
        in_slice[..Self::SIZE].copy_from_slice(&secret.to_bytes());

        PublicKey::from_ed25519_verifyingkey(&public)
    }

    pub(crate) fn sign_from_slice(sk: &[u8], m: &[u8]) -> <Self as KesSk<'a>>::Sig {
        let secret = EdSigningKey::from_bytes(sk.try_into().expect("Invalid sk size."));
        Sum0KesSig(secret.sign(m))
    }
}
impl KesSig for Sum0KesSig {
    fn verify(&self, _: u32, pk: &PublicKey, m: &[u8]) -> Result<(), Error> {
        let ed_pk = pk.as_ed25519()?;
        ed_pk.verify_strict(m, &self.0).map_err(Error::from)
    }
}

impl Sum0KesSig {
    /// Size of the KES signature with depth 0
    pub const SIZE: usize = SIGNATURE_LENGTH;

    /// Convert a byte array into a signature
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != Self::SIZE {
            return Err(Error::InvalidSecretKeySize(bytes.len()));
        }

        let mut signature = [0u8; Self::SIZE];
        signature.copy_from_slice(bytes);
        Ok(Self(EdSignature::from(signature)))
    }

    /// Return `Self` as a byte array.
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        self.0.to_bytes()
    }
}

#[derive(Debug)]
/// Single KES instance, which is a wrapper over ed25519.
pub struct Sum0CompactKes<'a>(pub(crate) &'a mut [u8]);

/// Singke KES Signature instance, which is a wrapper over ed25519.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde_as]
pub struct Sum0CompactKesSig(
    #[serde_as(as = "Bytes")] pub(crate) EdSignature,
    pub(crate) EdPublicKey,
);

impl Drop for Sum0CompactKes<'_> {
    fn drop(&mut self) {
        self.0.copy_from_slice(&[0u8; Self::SIZE + 4])
    }
}

impl<'a> KesSk<'a> for Sum0CompactKes<'a> {
    type Sig = Sum0CompactKesSig;
    const SIZE: usize = SECRET_KEY_LENGTH;

    fn keygen(key_buffer: &'a mut [u8], master_seed: &mut [u8]) -> (Self, PublicKey) {
        assert_eq!(key_buffer.len(), Self::SIZE + 4);
        assert_eq!(master_seed.len(), 32);

        let secret = EdSigningKey::from_bytes(
            &master_seed
                .try_into()
                .expect("Seed is defined with 32 bytes, so it won't fail."),
        );
        let public = (&secret).into();
        // We copy the secret key to the key buffer and we drop the secret key (which zeros de data)
        key_buffer[..32].copy_from_slice(&secret.to_bytes());
        drop(secret);
        master_seed.copy_from_slice(&[0u8; 32]);
        (
            Sum0CompactKes(key_buffer),
            PublicKey::from_ed25519_verifyingkey(&public),
        )
    }

    fn sign(&self, m: &[u8]) -> Sum0CompactKesSig {
        let secret = EdSigningKey::from_bytes(
            (&*self.0)
                .try_into()
                .expect("Seed is defined with 32 bytes, so it won't fail."),
        );
        let public = (&secret).into();
        Sum0CompactKesSig(secret.sign(m), public)
    }

    fn update(&mut self) -> Result<(), Error> {
        Err(Error::KeyCannotBeUpdatedMore)
    }

    fn get_period(&self) -> u32 {
        0
    }

    fn from_bytes(bytes: &'a mut [u8]) -> Result<Self, Error> {
        if bytes.len() != Self::SIZE + 4 {
            // We need to account for the seed
            return Err(Error::InvalidSecretKeySize(bytes.len()));
        }

        Ok(Self(bytes))
    }

    fn as_bytes(&self) -> &[u8] {
        self.0
    }
}

impl KesCompactSig for Sum0CompactKesSig {
    fn recompute(&self, _: u32, m: &[u8]) -> Result<PublicKey, Error> {
        self.1.verify_strict(m, &self.0)?;
        Ok(PublicKey(self.1.to_bytes()))
    }
}

impl<'a> Sum0CompactKes<'a> {
    pub(crate) fn update_slice(_: &mut [u8], _: u32) -> Result<(), Error> {
        Err(Error::KeyCannotBeUpdatedMore)
    }

    pub(crate) fn keygen_slice(in_slice: &mut [u8], opt_seed: Option<&mut [u8]>) -> PublicKey {
        let secret = if let Some(seed) = opt_seed {
            assert_eq!(in_slice.len(), Self::SIZE, "Input size is incorrect.");

            let sk =
                EdSigningKey::from_bytes(&seed.try_into().expect("Size of the seed is incorrect."));

            seed.copy_from_slice(&[0u8; 32]);
            sk
        } else {
            assert_eq!(
                in_slice.len(),
                Self::SIZE + Seed::SIZE,
                "Input size is incorrect."
            );

            let sk = EdSigningKey::from_bytes(
                &in_slice[Self::SIZE..]
                    .try_into()
                    .expect("Size of the seed is incorrect."),
            );

            in_slice[Self::SIZE..].copy_from_slice(&[0u8; 32]);
            sk
        };

        let public = (&secret).into();

        // We need to make this copies unfortunately by how the
        // underlying library behaves. Would be great to have a
        // EdPubKey from seed function.
        in_slice[..Self::SIZE].copy_from_slice(&secret.to_bytes());

        PublicKey::from_ed25519_verifyingkey(&public)
    }

    pub(crate) fn sign_from_slice(sk: &[u8], m: &[u8], _period: u32) -> <Self as KesSk<'a>>::Sig {
        let secret =
            EdSigningKey::from_bytes(sk.try_into().expect("Size of the seed is incorrect."));
        let public = (&secret).into();
        Sum0CompactKesSig(secret.sign(m), public)
    }
}

impl Sum0CompactKesSig {
    /// Size of the KES signature with depth 0
    pub const SIZE: usize = SIGNATURE_LENGTH + PUBLIC_KEY_LENGTH;

    /// Convert a byte array into a signature
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != Self::SIZE {
            return Err(Error::InvalidSecretKeySize(bytes.len()));
        }

        let mut signature = [0u8; SIGNATURE_LENGTH];
        let mut pk_bytes = [0u8; PUBLIC_KEY_LENGTH];
        signature.copy_from_slice(&bytes[..SIGNATURE_LENGTH]);
        pk_bytes.copy_from_slice(&bytes[SIGNATURE_LENGTH..]);
        let ed_key = EdPublicKey::from_bytes(&pk_bytes)?;
        Ok(Self(EdSignature::from(signature), ed_key))
    }

    /// Return `Self` as a byte array.
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        let mut output = [0u8; Self::SIZE];
        output[..SIGNATURE_LENGTH].copy_from_slice(&self.0.to_bytes());
        output[SIGNATURE_LENGTH..].copy_from_slice(self.1.as_bytes());
        output
    }
}
