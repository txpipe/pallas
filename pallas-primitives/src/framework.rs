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

    /// The same document as [`to_json`](Self::to_json), serialized to text.
    ///
    /// Recursive types override this to write the text directly: a
    /// `serde_json::Value` is itself recursive to serialize or drop, so for
    /// input whose depth is not under your control this is the safe entry
    /// point.
    fn to_json_string(&self) -> String {
        self.to_json().to_string()
    }
}
