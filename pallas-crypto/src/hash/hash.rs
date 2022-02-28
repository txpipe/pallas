use minicbor::{Decode, Encode};
use std::{fmt, ops::Deref, str::FromStr};

/// data that is a cryptographic [`struct@Hash`] of `BYTES` long.
///
/// Possible values with Cardano are 32 bytes long (block hash or transaction
/// hash). Or 28 bytes long (as used in addresses)
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash<const BYTES: usize>([u8; BYTES]);

impl<const BYTES: usize> Hash<BYTES> {
    #[inline]
    pub const fn new(bytes: [u8; BYTES]) -> Self {
        Self(bytes)
    }
}

impl<const BYTES: usize> From<[u8; BYTES]> for Hash<BYTES> {
    #[inline]
    fn from(bytes: [u8; BYTES]) -> Self {
        Self::new(bytes)
    }
}

impl<const BYTES: usize> AsRef<[u8]> for Hash<BYTES> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<const BYTES: usize> Deref for Hash<BYTES> {
    type Target = [u8; BYTES];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const BYTES: usize> PartialEq<[u8]> for Hash<BYTES> {
    fn eq(&self, other: &[u8]) -> bool {
        self.0.eq(other)
    }
}

impl<const BYTES: usize> fmt::Debug for Hash<BYTES> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&format!("Hash<{size}>", size = BYTES))
            .field(&hex::encode(self))
            .finish()
    }
}

impl<const BYTES: usize> fmt::Display for Hash<BYTES> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self))
    }
}

impl<const BYTES: usize> FromStr for Hash<BYTES> {
    type Err = hex::FromHexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = [0; BYTES];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self::new(bytes))
    }
}

impl<const BYTES: usize> Encode for Hash<BYTES> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(&self.0)?.ok()
    }
}

impl<'a, const BYTES: usize> Decode<'a> for Hash<BYTES> {
    fn decode(d: &mut minicbor::Decoder<'a>) -> Result<Self, minicbor::decode::Error> {
        let bytes = d.bytes()?;
        if bytes.len() == BYTES {
            let mut hash = [0; BYTES];
            hash.copy_from_slice(bytes);
            Ok(Self::new(hash))
        } else {
            // TODO: minicbor does not allow for expecting a specific size byte array
            //       (in fact cbor is not good at it at all anyway)
            Err(minicbor::decode::Error::message("Invalid hash size"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        let _digest: Hash<28> = "276fd18711931e2c0e21430192dbeac0e458093cd9d1fcd7210f64b3"
            .parse()
            .unwrap();

        let _digest: Hash<32> = "0d8d00cdd4657ac84d82f0a56067634a7adfdf43da41cb534bcaa45060973d21"
            .parse()
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn from_str_fail_1() {
        let _digest: Hash<28> = "27".parse().unwrap();
    }

    #[test]
    #[should_panic]
    fn from_str_fail_2() {
        let _digest: Hash<32> = "0d8d00cdd465".parse().unwrap();
    }
}
