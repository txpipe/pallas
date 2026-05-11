pub type Error = Box<dyn std::error::Error>;

use pallas_codec::minicbor::{Decode, Encode, decode, to_vec};

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

#[cfg(feature = "json")]
pub trait ToCanonicalJson {
    fn to_json(&self) -> serde_json::Value;
}
