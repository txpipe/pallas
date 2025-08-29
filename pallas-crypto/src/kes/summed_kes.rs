//! This module contains the macros to build the KES algorithms.
//! Tentative at making a recursive, and smaller code, which builds a key formed
//! by an array, allowing for a more granular memory management when calling the function.
//! The goal is to provide a similar construction to what is achieved in [sumed25519](./../sumed25519)
//! while maintaining code simplicity, and a smaller crate to facilitate audit and maintenance.

use crate::kes::common::{Depth, Seed};
use crate::kes::common::{PublicKey, INDIVIDUAL_SECRET_SIZE, PUBLIC_KEY_SIZE, SIGMA_SIZE};
use crate::kes::errors::Error;
use crate::kes::single_kes::{Sum0CompactKes, Sum0CompactKesSig, Sum0Kes, Sum0KesSig};
use crate::kes::traits::{KesCompactSig, KesSig, KesSk};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use zeroize::Zeroize;

macro_rules! sum_kes {
    ($name:ident, $signame:ident, $sk:ident, $sigma:ident, $depth:expr, $doc:expr) => {
        #[derive(Debug)]
        #[doc=$doc]
        pub struct $name<'a>(&'a mut [u8]);

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        /// Structure that represents a KES signature.
        pub struct $signame {
            sigma: $sigma,
            lhs_pk: PublicKey,
            rhs_pk: PublicKey,
        }

        impl<'a> Drop for $name<'a> {
            fn drop(&mut self) {
                self.0.zeroize();
            }
        }

        // First we implement the KES traits.
        impl<'a> KesSk<'a> for $name<'a> {
            type Sig = $signame;
            const SIZE: usize =
                INDIVIDUAL_SECRET_SIZE + $depth * 32 + $depth * (PUBLIC_KEY_SIZE * 2);
            fn keygen(key_buffer: &'a mut [u8], seed: &'a mut [u8]) -> (Self, PublicKey) {
                assert_eq!(key_buffer.len(), Self::SIZE + 4);
                assert_eq!(seed.len(), 32);

                let pk = Self::keygen_slice(&mut key_buffer[..Self::SIZE], Some(seed));

                // We write the period the the main data.
                key_buffer[Self::SIZE..].copy_from_slice(&0u32.to_be_bytes());

                (Self(key_buffer), pk)
            }

            fn sign(&self, m: &[u8]) -> Self::Sig {
                Self::sign_from_slice(self.as_bytes(), m)
            }

            fn update(&mut self) -> Result<(), Error> {
                let mut u32_bytes = [0u8; 4];
                u32_bytes.copy_from_slice(&self.0[Self::SIZE..]);
                let period = u32::from_be_bytes(u32_bytes);

                Self::update_slice(&mut self.0[..Self::SIZE], period)?;

                self.0[Self::SIZE..].copy_from_slice(&(period + 1).to_be_bytes());
                Ok(())
            }

            fn get_period(&self) -> u32 {
                let mut u32_bytes = [0u8; 4];
                u32_bytes.copy_from_slice(&self.0[Self::SIZE..]);
                u32::from_be_bytes(u32_bytes)
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

        impl KesSig for $signame {
            fn verify(&self, period: u32, pk: &PublicKey, m: &[u8]) -> Result<(), Error> {
                if &self.lhs_pk.hash_pair(&self.rhs_pk) != pk {
                    return Err(Error::InvalidHashComparison);
                }

                if period < Depth($depth).half() {
                    self.sigma.verify(period, &self.lhs_pk, m)?;
                } else {
                    self.sigma
                        .verify(period - &Depth($depth).half(), &self.rhs_pk, m)?
                }

                Ok(())
            }
        }

        impl<'a> $name<'a> {
            pub(crate) fn update_slice(key_slice: &mut [u8], period: u32) -> Result<(), Error> {
                if period + 1 == Depth($depth).total() {
                    return Err(Error::KeyCannotBeUpdatedMore);
                }

                match (period + 1).cmp(&Depth($depth).half()) {
                    Ordering::Less => $sk::update_slice(&mut key_slice[..$sk::SIZE], period)?,
                    Ordering::Equal => {
                        $sk::keygen_slice(&mut key_slice[..$sk::SIZE + 32], None);
                    }
                    Ordering::Greater => $sk::update_slice(
                        &mut key_slice[..$sk::SIZE],
                        period - &Depth($depth).half(),
                    )?,
                }

                Ok(())
            }

            pub(crate) fn keygen_slice(
                in_slice: &mut [u8],
                opt_seed: Option<&mut [u8]>,
            ) -> PublicKey {
                let (mut r0, mut seed) = if let Some(in_seed) = opt_seed {
                    assert_eq!(in_slice.len(), Self::SIZE, "Input size is incorrect.");
                    assert_eq!(in_seed.len(), Seed::SIZE, "Input seed is incorrect.");
                    Seed::split_slice(in_seed)
                } else {
                    assert_eq!(
                        in_slice.len(),
                        Self::SIZE + Seed::SIZE,
                        "Input size is incorrect."
                    );
                    Seed::split_slice(&mut in_slice[Self::SIZE..])
                };

                // We copy the seed before overwriting with zeros (in the `keygen` call).
                in_slice[$sk::SIZE..$sk::SIZE + 32].copy_from_slice(&seed);
                // Buffer for temp key
                let mut temp_buffer = [0u8; $sk::SIZE + 4];

                let pk_0 = $sk::keygen_slice(&mut in_slice[..$sk::SIZE], Some(&mut r0));
                let (_, pk_1) = $sk::keygen(&mut temp_buffer, &mut seed);
                temp_buffer[..].copy_from_slice(&[0u8; $sk::SIZE + 4]);

                let pk = pk_0.hash_pair(&pk_1);

                // We write the pkeys to the main data.
                in_slice[$sk::SIZE + 32..$sk::SIZE + 64].copy_from_slice(pk_0.as_bytes());
                in_slice[$sk::SIZE + 64..$sk::SIZE + 96].copy_from_slice(pk_1.as_bytes());

                pk
            }

            pub(crate) fn sign_from_slice(sk: &[u8], m: &[u8]) -> <Self as KesSk<'a>>::Sig {
                let sigma = $sk::sign_from_slice(&sk[..$sk::SIZE], m);

                let lhs_pk = PublicKey::from_bytes(&sk[$sk::SIZE + 32..$sk::SIZE + 64])
                    .expect("Won't fail as slice has size 32");
                let rhs_pk = PublicKey::from_bytes(&sk[$sk::SIZE + 64..$sk::SIZE + 96])
                    .expect("Won't fail as slice has size 32");
                $signame {
                    sigma,
                    lhs_pk,
                    rhs_pk,
                }
            }

            /// Convert KES sk to PublicKey
            pub fn to_pk(&self) -> PublicKey {
                let pk0 = PublicKey::from_bytes(
                    &self.0[Self::SIZE - PUBLIC_KEY_SIZE * 2..Self::SIZE - PUBLIC_KEY_SIZE],
                )
                .expect("Key size is valid.");
                let pk1 = PublicKey::from_bytes(&self.0[Self::SIZE - PUBLIC_KEY_SIZE..Self::SIZE])
                    .expect("Key size is valid");

                pk0.hash_pair(&pk1)
            }

            #[cfg(feature = "sk_clone_enabled")]
            /// Clone the secret data. this should only be used for testing
            pub fn clone_sk(&self) -> Vec<u8> {
                let mut bytes = vec![0u8; Self::SIZE + 4];
                bytes.copy_from_slice(self.0);
                bytes
            }
        }

        impl $signame {
            /// Byte size of the signature
            pub const SIZE: usize = SIGMA_SIZE + $depth * (PUBLIC_KEY_SIZE * 2);

            /// Convert the slice of bytes into `Self`.
            ///
            /// # Errors
            /// The function fails if
            /// * `bytes.len()` is not of the expected size
            /// * the bytes in the expected positions of the signature do not represent a valid
            ///   signature
            pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
                if bytes.len() != Self::SIZE {
                    return Err(Error::InvalidSignatureSize(bytes.len()));
                }

                let sigma = $sigma::from_bytes(&bytes[..$sigma::SIZE])?;
                let lhs_pk =
                    PublicKey::from_bytes(&bytes[$sigma::SIZE..$sigma::SIZE + PUBLIC_KEY_SIZE])?;
                let rhs_pk = PublicKey::from_bytes(
                    &bytes[$sigma::SIZE + PUBLIC_KEY_SIZE..$sigma::SIZE + 2 * PUBLIC_KEY_SIZE],
                )?;
                Ok(Self {
                    sigma,
                    lhs_pk,
                    rhs_pk,
                })
            }

            /// Convert `Self` into it's byte representation. In particular, the encoding returns
            /// the following array of size `Self::SIZE`:
            /// ( self.sigma || self.lhs_pk || self.rhs_pk )
            pub fn to_bytes(&self) -> [u8; Self::SIZE] {
                let mut data = [0u8; Self::SIZE];
                data[..$sigma::SIZE].copy_from_slice(&self.sigma.to_bytes());
                data[$sigma::SIZE..$sigma::SIZE + PUBLIC_KEY_SIZE]
                    .copy_from_slice(self.lhs_pk.as_ref());
                data[$sigma::SIZE + PUBLIC_KEY_SIZE..$sigma::SIZE + 2 * PUBLIC_KEY_SIZE]
                    .copy_from_slice(self.rhs_pk.as_ref());

                data
            }
        }
    };
}
macro_rules! sum_compact_kes {
    ($name:ident, $signame:ident, $sk:ident, $sigma:ident, $depth:expr, $doc:expr) => {
        #[derive(Debug)]
        #[doc=$doc]
        pub struct $name<'a>(&'a mut [u8]);

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        /// Structure that represents a KES signature.
        pub struct $signame {
            sigma: $sigma,
            pk: PublicKey,
        }

        // First we implement the KES traits.
        impl<'a> KesSk<'a> for $name<'a> {
            type Sig = $signame;
            const SIZE: usize =
                INDIVIDUAL_SECRET_SIZE + $depth * 32 + $depth * (PUBLIC_KEY_SIZE * 2);

            /// Function that takes a mutable
            fn keygen(key_buffer: &'a mut [u8], master_seed: &mut [u8]) -> (Self, PublicKey) {
                assert_eq!(key_buffer.len(), Self::SIZE + 4);
                assert_eq!(master_seed.len(), 32);

                let pk = Self::keygen_slice(&mut key_buffer[..Self::SIZE], Some(master_seed));

                // We write the period the the main data.
                key_buffer[Self::SIZE..].copy_from_slice(&0u32.to_be_bytes());

                (Self(key_buffer), pk)
            }

            fn sign(&self, m: &[u8]) -> Self::Sig {
                let mut u32_bytes = [0u8; 4];
                u32_bytes.copy_from_slice(&self.0[Self::SIZE..]);
                let period = u32::from_be_bytes(u32_bytes);

                Self::sign_from_slice(self.as_bytes(), m, period)
            }

            fn update(&mut self) -> Result<(), Error> {
                let mut u32_bytes = [0u8; 4];
                u32_bytes.copy_from_slice(&self.0[Self::SIZE..]);
                let period = u32::from_be_bytes(u32_bytes);

                Self::update_slice(&mut self.0[..Self::SIZE], period)?;

                self.0[Self::SIZE..].copy_from_slice(&(period + 1).to_be_bytes());
                Ok(())
            }

            fn get_period(&self) -> u32 {
                let mut u32_bytes = [0u8; 4];
                u32_bytes.copy_from_slice(&self.0[Self::SIZE..]);
                u32::from_be_bytes(u32_bytes)
            }

            fn from_bytes(bytes: &'a mut [u8]) -> Result<Self, Error> {
                if bytes.len() != Self::SIZE + 4 {
                    return Err(Error::InvalidSecretKeySize(bytes.len()));
                }

                Ok(Self(bytes))
            }

            fn as_bytes(&self) -> &[u8] {
                self.0
            }
        }

        impl KesCompactSig for $signame {
            fn recompute(&self, period: u32, m: &[u8]) -> Result<PublicKey, Error> {
                if period < Depth($depth).half() {
                    let recomputed_key = self.sigma.recompute(period, m)?;
                    Ok(recomputed_key.hash_pair(&self.pk))
                } else {
                    let recomputed_key = self.sigma.recompute(period - &Depth($depth).half(), m)?;
                    Ok(self.pk.hash_pair(&recomputed_key))
                }
            }
        }

        impl<'a> Drop for $name<'a> {
            fn drop(&mut self) {
                self.0.copy_from_slice(&[0u8; Self::SIZE + 4])
            }
        }

        impl<'a> $name<'a> {
            pub(crate) fn update_slice(key_slice: &mut [u8], period: u32) -> Result<(), Error> {
                if period + 1 == Depth($depth).total() {
                    return Err(Error::KeyCannotBeUpdatedMore);
                }

                match (period + 1).cmp(&Depth($depth).half()) {
                    Ordering::Less => $sk::update_slice(&mut key_slice[..$sk::SIZE], period)?,
                    Ordering::Equal => {
                        $sk::keygen_slice(&mut key_slice[..$sk::SIZE + 32], None);
                    }
                    Ordering::Greater => $sk::update_slice(
                        &mut key_slice[..$sk::SIZE],
                        period - &Depth($depth).half(),
                    )?,
                }

                Ok(())
            }

            pub(crate) fn keygen_slice(
                in_slice: &mut [u8],
                opt_seed: Option<&mut [u8]>,
            ) -> PublicKey {
                let (mut r0, mut seed) = if let Some(in_seed) = opt_seed {
                    assert_eq!(in_slice.len(), Self::SIZE, "Size of the seed is incorrect.");
                    assert_eq!(in_seed.len(), Seed::SIZE, "Input seed is incorrect.");
                    Seed::split_slice(in_seed)
                } else {
                    assert_eq!(
                        in_slice.len(),
                        Self::SIZE + Seed::SIZE,
                        "Input size is incorrect."
                    );
                    Seed::split_slice(&mut in_slice[Self::SIZE..])
                };

                in_slice[$sk::SIZE..$sk::SIZE + 32].copy_from_slice(&seed);
                // Buffer for temp key
                let mut temp_buffer = [0u8; $sk::SIZE + 4];

                let pk_0 = $sk::keygen_slice(&mut in_slice[..$sk::SIZE], Some(&mut r0));
                let (_, pk_1) = $sk::keygen(&mut temp_buffer, &mut seed);
                temp_buffer[..].copy_from_slice(&[0u8; $sk::SIZE + 4]);

                let pk = pk_0.hash_pair(&pk_1);

                // We write the keys to the main data.
                in_slice[$sk::SIZE + 32..$sk::SIZE + 64].copy_from_slice(pk_0.as_bytes());
                in_slice[$sk::SIZE + 64..$sk::SIZE + 96].copy_from_slice(pk_1.as_bytes());

                pk
            }

            pub(crate) fn sign_from_slice(
                sk: &[u8],
                m: &[u8],
                period: u32,
            ) -> <Self as KesSk<'a>>::Sig {
                let t0 = Depth($depth).half();
                let mut pk_bytes = [0u8; 32];
                let sigma = if period < t0 {
                    pk_bytes.copy_from_slice(&sk[$sk::SIZE + 64..$sk::SIZE + 96]);
                    $sk::sign_from_slice(&sk[..$sk::SIZE], m, period)
                } else {
                    pk_bytes.copy_from_slice(&sk[$sk::SIZE + 32..$sk::SIZE + 64]);
                    $sk::sign_from_slice(&sk[..$sk::SIZE], m, period - t0)
                };

                let pk = PublicKey::from_bytes(&pk_bytes).expect("Won't fail as slice has size 32");
                $signame { sigma, pk }
            }

            /// Convert KES key to public key
            pub fn to_pk(&self) -> PublicKey {
                let pk0 = PublicKey::from_bytes(
                    &self.0[Self::SIZE - PUBLIC_KEY_SIZE * 2..Self::SIZE - PUBLIC_KEY_SIZE],
                )
                .expect("Key size is valid.");
                let pk1 = PublicKey::from_bytes(&self.0[Self::SIZE - PUBLIC_KEY_SIZE..Self::SIZE])
                    .expect("Key size is valid");

                pk0.hash_pair(&pk1)
            }

            #[cfg(feature = "sk_clone_enabled")]
            /// Clone the secret data. this should only be used for testing
            pub fn clone_sk(&self) -> Vec<u8> {
                let mut bytes = vec![0u8; Self::SIZE + 4];
                bytes.copy_from_slice(self.0);
                bytes
            }
        }

        impl $signame {
            /// Byte size of the signature
            pub const SIZE: usize = SIGMA_SIZE + ($depth + 1) * PUBLIC_KEY_SIZE;

            /// Convert the slice of bytes into `Self`.
            ///
            /// # Errors
            /// The function fails if
            /// * `bytes.len()` is not of the expected size
            /// * the bytes in the expected positions of the signature do not represent a valid
            ///   signature
            pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
                if bytes.len() != Self::SIZE {
                    return Err(Error::InvalidSignatureSize(bytes.len()));
                }

                let sigma = $sigma::from_bytes(&bytes[..$sigma::SIZE])?;
                let pk =
                    PublicKey::from_bytes(&bytes[$sigma::SIZE..$sigma::SIZE + PUBLIC_KEY_SIZE])?;

                Ok(Self { sigma, pk })
            }

            /// Convert `Self` into it's byte representation. In particular, the encoding returns
            /// the following array of size `Self::SIZE`:
            /// ( self.sigma || self.lhs_pk || self.rhs_pk )
            pub fn to_bytes(&self) -> [u8; Self::SIZE] {
                let mut data = [0u8; Self::SIZE];
                data[..$sigma::SIZE].copy_from_slice(&self.sigma.to_bytes());
                data[$sigma::SIZE..$sigma::SIZE + PUBLIC_KEY_SIZE]
                    .copy_from_slice(self.pk.as_ref());

                data
            }
        }
    };
}

sum_kes!(
    Sum1Kes,
    Sum1KesSig,
    Sum0Kes,
    Sum0KesSig,
    1,
    "KES implementation with depth 1"
);
sum_kes!(
    Sum2Kes,
    Sum2KesSig,
    Sum1Kes,
    Sum1KesSig,
    2,
    "KES implementation with depth 2"
);
sum_kes!(
    Sum3Kes,
    Sum3KesSig,
    Sum2Kes,
    Sum2KesSig,
    3,
    "KES implementation with depth 3"
);
sum_kes!(
    Sum4Kes,
    Sum4KesSig,
    Sum3Kes,
    Sum3KesSig,
    4,
    "KES implementation with depth 4"
);
sum_kes!(
    Sum5Kes,
    Sum5KesSig,
    Sum4Kes,
    Sum4KesSig,
    5,
    "KES implementation with depth 5"
);
sum_kes!(
    Sum6Kes,
    Sum6KesSig,
    Sum5Kes,
    Sum5KesSig,
    6,
    "KES implementation with depth 6"
);
sum_kes!(
    Sum7Kes,
    Sum7KesSig,
    Sum6Kes,
    Sum6KesSig,
    7,
    "KES implementation with depth 7"
);

sum_compact_kes!(
    Sum1CompactKes,
    Sum1CompactKesSig,
    Sum0CompactKes,
    Sum0CompactKesSig,
    1,
    "KES implementation with depth 1"
);
sum_compact_kes!(
    Sum2CompactKes,
    Sum2CompactKesSig,
    Sum1CompactKes,
    Sum1CompactKesSig,
    2,
    "KES implementation with depth 2"
);
sum_compact_kes!(
    Sum3CompactKes,
    Sum3CompactKesSig,
    Sum2CompactKes,
    Sum2CompactKesSig,
    3,
    "KES implementation with depth 3"
);
sum_compact_kes!(
    Sum4CompactKes,
    Sum4CompactKesSig,
    Sum3CompactKes,
    Sum3CompactKesSig,
    4,
    "KES implementation with depth 4"
);
sum_compact_kes!(
    Sum5CompactKes,
    Sum5CompactKesSig,
    Sum4CompactKes,
    Sum4CompactKesSig,
    5,
    "KES implementation with depth 5"
);
sum_compact_kes!(
    Sum6CompactKes,
    Sum6CompactKesSig,
    Sum5CompactKes,
    Sum5CompactKesSig,
    6,
    "KES implementation with depth 6"
);
sum_compact_kes!(
    Sum7CompactKes,
    Sum7CompactKesSig,
    Sum6CompactKes,
    Sum6CompactKesSig,
    7,
    "KES implementation with depth 7"
);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn buff_single() {
        let mut skey_buffer = [0u8; Sum1Kes::SIZE + 4];
        let mut seed = [0u8; Seed::SIZE];
        let (mut skey, pkey) = Sum1Kes::keygen(&mut skey_buffer, &mut seed);
        let dummy_message = b"tilin";
        let sigma = skey.sign(dummy_message);

        assert_eq!(skey.get_period(), 0);

        assert!(sigma.verify(0, &pkey, dummy_message).is_ok());

        // Key can be updated once
        assert!(skey.update().is_ok());
    }

    #[test]
    fn buff_4() {
        let mut skey_buffer = [0u8; Sum4Kes::SIZE + 4];
        let mut seed = [0u8; Seed::SIZE];
        let (mut skey, pkey) = Sum4Kes::keygen(&mut skey_buffer, &mut seed);
        let dummy_message = b"tilin";
        let sigma = skey.sign(dummy_message);

        assert!(sigma.verify(0, &pkey, dummy_message).is_ok());

        // Key can be updated 2^4 - 1 times
        for _ in 0..15 {
            assert!(skey.update().is_ok());
        }

        assert_eq!(skey.get_period(), 15);

        let sigma_15 = skey.sign(dummy_message);
        assert!(sigma_15.verify(15, &pkey, dummy_message).is_ok())
    }

    #[test]
    fn buff_compact_single() {
        let mut skey_buffer = [0u8; Sum1CompactKes::SIZE + 4];
        let mut seed = [0u8; Seed::SIZE];
        let (mut skey, pkey) = Sum1CompactKes::keygen(&mut skey_buffer, &mut seed);
        let dummy_message = b"tilin";
        let sigma = skey.sign(dummy_message);

        assert_eq!(skey.get_period(), 0);

        assert!(sigma.verify(0, &pkey, dummy_message).is_ok());

        // Key can be updated once
        assert!(skey.update().is_ok());
    }

    #[test]
    fn buff_compact_4() {
        let mut skey_buffer = [0u8; Sum4CompactKes::SIZE + 4];
        let mut seed = [0u8; Seed::SIZE];
        let (mut skey, pkey) = Sum4CompactKes::keygen(&mut skey_buffer, &mut seed);
        let dummy_message = b"tilin";
        let sigma = skey.sign(dummy_message);

        assert!(sigma.verify(0, &pkey, dummy_message).is_ok());

        // Key can be updated 2^4 - 1 times
        for _ in 0..15 {
            assert!(skey.update().is_ok());
        }

        assert_eq!(skey.get_period(), 15);
    }

    #[test]
    fn test_to_pk() {
        let mut skey_buffer = [0u8; Sum4CompactKes::SIZE + 4];
        let mut seed = [0u8; Seed::SIZE];
        let (skey, pkey) = Sum4CompactKes::keygen(&mut skey_buffer, &mut seed);

        assert_eq!(skey.to_pk(), pkey);

        let mut skey_buffer = [0u8; Sum4Kes::SIZE + 4];
        let mut seed = [0u8; Seed::SIZE];
        let (skey, pkey) = Sum4Kes::keygen(&mut skey_buffer, &mut seed);

        assert_eq!(skey.to_pk(), pkey);
    }
}

#[cfg(test)]
mod test_serde {
    use super::*;

    #[test]
    fn test_serde_1() {
        let mut skey_buffer = [0u8; Sum1Kes::SIZE + 4];
        let mut seed = [0u8; 32];
        let (skey, pkey) = Sum1Kes::keygen(&mut skey_buffer, &mut seed);

        let pkey_bytes = serde_json::to_string(&pkey).unwrap();
        let deser_pkey: PublicKey = serde_json::from_str(&pkey_bytes).unwrap();

        assert_eq!(pkey, deser_pkey);

        let dummy_message = b"tolon";
        let sigma = skey.sign(dummy_message);

        let sigma_bytes = serde_json::to_string(&sigma).unwrap();
        let deser_sigma: Sum1KesSig = serde_json::from_str(&sigma_bytes).unwrap();

        assert_eq!(sigma, deser_sigma);
        assert!(deser_sigma.verify(0, &pkey, dummy_message).is_ok());

        let mut skey_buffer = [0u8; Sum1CompactKes::SIZE + 4];
        let mut seed = [0u8; 32];
        let (skey1, pkey) = Sum1CompactKes::keygen(&mut skey_buffer, &mut seed);

        let pkey_bytes = serde_json::to_string(&pkey).unwrap();
        let deser_pkey: PublicKey = serde_json::from_str(&pkey_bytes).unwrap();

        assert_eq!(pkey, deser_pkey);

        let dummy_message = b"tolon";
        let sigma = skey1.sign(dummy_message);

        let sigma_bytes = serde_json::to_string(&sigma).unwrap();
        let deser_sigma: Sum1CompactKesSig = serde_json::from_str(&sigma_bytes).unwrap();

        assert_eq!(sigma, deser_sigma);
        assert!(deser_sigma.verify(0, &pkey, dummy_message).is_ok());
    }

    #[test]
    fn test_serde_4() {
        let mut skey_buffer = [0u8; Sum4Kes::SIZE + 4];
        let mut seed = [0u8; 32];
        let (skey, pkey) = Sum4Kes::keygen(&mut skey_buffer, &mut seed);

        let pkey_bytes = serde_json::to_string(&pkey).unwrap();
        let deser_pkey: PublicKey = serde_json::from_str(&pkey_bytes).unwrap();

        assert_eq!(pkey, deser_pkey);

        let dummy_message = b"tolon";
        let sigma = skey.sign(dummy_message);

        let sigma_bytes = serde_json::to_string(&sigma).unwrap();
        let deser_sigma: Sum4KesSig = serde_json::from_str(&sigma_bytes).unwrap();

        assert_eq!(sigma, deser_sigma);
        assert!(deser_sigma.verify(0, &pkey, dummy_message).is_ok());

        let mut skey_buffer = [0u8; Sum4CompactKes::SIZE + 4];
        let mut seed = [0u8; 32];
        let (skey, pkey) = Sum4CompactKes::keygen(&mut skey_buffer, &mut seed);

        let pkey_bytes = serde_json::to_string(&pkey).unwrap();
        let deser_pkey: PublicKey = serde_json::from_str(&pkey_bytes).unwrap();

        assert_eq!(pkey, deser_pkey);

        let dummy_message = b"tolon";
        let sigma = skey.sign(dummy_message);

        let sigma_bytes = serde_json::to_string(&sigma).unwrap();
        let deser_sigma: Sum4CompactKesSig = serde_json::from_str(&sigma_bytes).unwrap();

        assert_eq!(sigma, deser_sigma);
        assert!(deser_sigma.verify(0, &pkey, dummy_message).is_ok());
    }

    #[test]
    fn test_serde_6() {
        let mut skey_buffer = [0u8; Sum6Kes::SIZE + 4];
        let mut seed = [0u8; 32];
        let (skey, pkey) = Sum6Kes::keygen(&mut skey_buffer, &mut seed);

        let pkey_bytes = serde_json::to_string(&pkey).unwrap();
        let deser_pkey: PublicKey = serde_json::from_str(&pkey_bytes).unwrap();

        assert_eq!(pkey, deser_pkey);

        let dummy_message = b"tolon";
        let sigma = skey.sign(dummy_message);

        let sigma_bytes = serde_json::to_string(&sigma).unwrap();
        let deser_sigma: Sum6KesSig = serde_json::from_str(&sigma_bytes).unwrap();

        assert_eq!(sigma, deser_sigma);
        assert!(deser_sigma.verify(0, &pkey, dummy_message).is_ok());

        let mut skey_buffer = [0u8; Sum6CompactKes::SIZE + 4];
        let mut seed = [0u8; 32];
        let (skey, pkey) = Sum6CompactKes::keygen(&mut skey_buffer, &mut seed);

        let pkey_bytes = serde_json::to_string(&pkey).unwrap();
        let deser_pkey: PublicKey = serde_json::from_str(&pkey_bytes).unwrap();

        assert_eq!(pkey, deser_pkey);

        let dummy_message = b"tolon";
        let sigma = skey.sign(dummy_message);

        let sigma_bytes = serde_json::to_string(&sigma).unwrap();
        let deser_sigma: Sum6CompactKesSig = serde_json::from_str(&sigma_bytes).unwrap();

        assert_eq!(sigma, deser_sigma);
        assert!(deser_sigma.verify(0, &pkey, dummy_message).is_ok());
    }
}
