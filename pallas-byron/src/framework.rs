pub type Error = Box<dyn std::error::Error>;

pub trait Fragment<'a>
where
    Self: Sized,
{
    fn encode_fragment(&self) -> Result<Vec<u8>, Error>;
    fn decode_fragment(bytes: &'a [u8]) -> Result<Self, Error>;
}

impl<'a, T> Fragment<'a> for T
where
    T: minicbor::Encode + minicbor::Decode<'a> + Sized,
{
    fn encode_fragment(&self) -> Result<Vec<u8>, Error> {
        minicbor::to_vec(self).map_err(|e| e.into())
    }

    fn decode_fragment(bytes: &'a [u8]) -> Result<Self, Error> {
        minicbor::decode(bytes).map_err(|e| e.into())
    }
}
