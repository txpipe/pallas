pub type Error = Box<dyn std::error::Error>;

use pallas_codec::minicbor::{decode, to_vec, Decode, Encode};

pub trait Fragment<'a>
where
    Self: Sized,
{
    fn encode_fragment(&self) -> Result<Vec<u8>, Error>;
    fn decode_fragment(bytes: &'a [u8]) -> Result<Self, Error>;
}

impl<'a, T> Fragment<'a> for T
where
    T: Encode<()> + Decode<'a, ()> + Sized,
{
    fn encode_fragment(&self) -> Result<Vec<u8>, Error> {
        to_vec(self).map_err(|e| e.into())
    }

    fn decode_fragment(bytes: &'a [u8]) -> Result<Self, Error> {
        decode(bytes).map_err(|e| e.into())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Era {
    Byron,
    Shelley,
    Allegra, // time-locks
    Mary,    // multi-assets
    Alonzo,  // smart-contracts
}

#[cfg(feature = "json")]
pub trait ToCanonicalJson {
    fn to_json(&self) -> serde_json::Value;
}

pub trait ToHash<const BYTES: usize> {
    fn to_hash(&self) -> pallas_crypto::hash::Hash<BYTES>;
}
