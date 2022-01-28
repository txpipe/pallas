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
        let mut buf = Vec::new();
        {
            let mut encoder = minicbor::Encoder::new(&mut buf);
            encoder.encode(self).expect("error encoding");
        }

        Ok(buf)
    }

    fn decode_fragment(bytes: &'a [u8]) -> Result<Self, Error> {
        let mut decoder = minicbor::Decoder::new(bytes);
        let out = decoder.decode().expect("error decoding");

        Ok(out)
    }
}
