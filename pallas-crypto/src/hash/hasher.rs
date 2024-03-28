use crate::hash::Hash;
use cryptoxide::blake2b::Blake2b;
use pallas_codec::minicbor;

/// handy method to create a hash of given `SIZE` bit size.
///
/// The hash algorithm is `Blake2b` and the constant parameter is
/// the number of bits to generate. Good values are `256` or `224` for
/// Cardano.
///
/// # Generate a cryptographic hash with Blake2b 256
///
/// The following will generate a 32 bytes digest output
///
/// ```
/// # use pallas_crypto::hash::Hasher;
///
/// let mut hasher = Hasher::<256>::new();
/// hasher.input(b"My transaction");
///
/// let digest = hasher.finalize();
/// # assert_eq!(
/// #   "0d8d00cdd4657ac84d82f0a56067634a7adfdf43da41cb534bcaa45060973d21",
/// #   hex::encode(digest)
/// # );
/// ```
///
/// # Generate a cryptographic hash with Blake2b 224
///
/// The following will generate a 28 bytes digest output. This is used
/// to generate the hash of public keys for addresses.
///
/// ```
/// # use pallas_crypto::hash::Hasher;
///
/// let digest = Hasher::<224>::hash(b"My Public Key");
/// # assert_eq!(
/// #   "c123c9bc0e9e31a20a4aa23518836ec5fb54bdc85735c56b38eb79a5",
/// #   hex::encode(digest)
/// # );
/// ```
pub struct Hasher<const BITS: usize>(Blake2b);

impl<const BITS: usize> Hasher<BITS> {
    /// update the [`Hasher`] with the given inputs
    #[inline]
    pub fn input(&mut self, bytes: &[u8]) {
        use cryptoxide::digest::Digest as _;
        self.0.input(bytes);
    }
}

macro_rules! common_hasher {
    ($size:literal) => {
        impl Hasher<$size> {
            /// create a new [`Hasher`]
            #[inline]
            pub fn new() -> Self {
                Self(Blake2b::new($size / 8))
            }

            /// convenient function to directly generate the hash
            /// of the given bytes without creating the intermediary
            /// types [`Hasher`] and calling [`Hasher::input`].
            #[inline]
            pub fn hash(bytes: &[u8]) -> Hash<{ $size / 8 }> {
                let mut hasher = Self::new();
                hasher.input(bytes);
                hasher.finalize()
            }

            #[inline]
            pub fn hash_tagged(bytes: &[u8], tag: u8) -> Hash<{ $size / 8 }> {
                let mut hasher = Self::new();
                hasher.input(&[tag]);
                hasher.input(bytes);
                hasher.finalize()
            }

            /// convenient function to directly generate the hash
            /// of the given [minicbor::Encode] data object
            #[inline]
            pub fn hash_cbor(data: &impl minicbor::Encode<()>) -> Hash<{ $size / 8 }> {
                let mut hasher = Self::new();
                let () = minicbor::encode(data, &mut hasher).expect("Infallible");
                hasher.finalize()
            }

            #[inline]
            pub fn hash_tagged_cbor(
                data: &impl minicbor::Encode<()>,
                tag: u8,
            ) -> Hash<{ $size / 8 }> {
                let mut hasher = Self::new();
                hasher.input(&[tag]);
                let () = minicbor::encode(data, &mut hasher).expect("Infallible");
                hasher.finalize()
            }

            /// consume the [`Hasher`] and returns the computed digest
            pub fn finalize(mut self) -> Hash<{ $size / 8 }> {
                use cryptoxide::digest::Digest as _;
                let mut hash = [0; $size / 8];
                self.0.result(&mut hash);
                Hash::new(hash)
            }
        }

        impl Default for Hasher<$size> {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

common_hasher!(160);
common_hasher!(224);
common_hasher!(256);

/*
TODO: somehow the `minicbor::Write` does not allow to implement this
      version of the trait and to automatically have the impl of the
      other one automatically derived by default.

impl<const BITS: usize> Write for Hasher<BITS> {
    type Error = std::convert::Infallible;

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.input(buf);
        Ok(())
    }
}
*/

impl<'a, const BITS: usize> minicbor::encode::Write for &'a mut Hasher<BITS> {
    type Error = std::convert::Infallible;

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.input(buf);
        Ok(())
    }
}
