use minicbor::encode::Write;

/// Shared re-export of minicbor lib across all Pallas
pub use minicbor;

/// Round-trip friendly common helper structs
pub mod utils;

pub trait Fragment: Sized {
    fn read_cbor(buffer: &[u8]) -> Result<Self, minicbor::decode::Error>;
    fn write_cbor<W: Write>(&self, write: W) -> Result<(), minicbor::encode::Error<W::Error>>;
}

pub trait DecodeOwned: for<'b> minicbor::Decode<'b> {}

impl<T> DecodeOwned for T where T: for<'b> minicbor::Decode<'b> {}

impl<T> Fragment for T
where
    T: DecodeOwned + minicbor::Encode,
{
    fn read_cbor(buffer: &[u8]) -> Result<Self, minicbor::decode::Error> {
        minicbor::decode(buffer)
    }

    fn write_cbor<W: Write>(&self, write: W) -> Result<(), minicbor::encode::Error<W::Error>> {
        minicbor::encode(self, write)
    }
}
