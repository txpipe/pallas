use minicbor::encode::Write;

/// Shared re-export of minicbor lib across all Pallas
pub use minicbor;

/// Round-trip friendly common helper structs
pub mod utils;

pub trait Fragment: Sized {
    fn from_cbor(buffer: &[u8]) -> Result<Self, minicbor::decode::Error>;
    fn into_cbor<W: Write>(&self, write: W) -> Result<(), minicbor::encode::Error<W::Error>>;
}

#[macro_export]
macro_rules! impl_fragment {
    ($Struct:ty) => {
        impl $crate::Fragment for $Struct {
            fn from_cbor(buffer: &[u8]) -> Result<Self, decode::Error> {
                $crate::minicbor::decode(buffer)
            }

            fn into_cbor<W: encode::Write>(&self, write: W) -> Result<(), encode::Error<W::Error>> {
                $crate::minicbor::encode(self, write)
            }
        }
    };
}
