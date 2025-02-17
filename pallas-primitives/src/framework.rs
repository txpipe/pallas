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

#[cfg(feature = "json")]
pub trait ToCanonicalJson {
    fn to_json(&self) -> serde_json::Value;
}

macro_rules! codec_by_datatype {
    (
        $enum_name:ident,
        $( $one_f:ident => $( $cbortype:ident )|* ),*,
        ($( $many_f:ident => $( $vars:ident:$types:ty | $( $cbortypes:ident )|*),+ )?)
    ) => {
        impl<'b, C> minicbor::decode::Decode<'b, C> for $enum_name {
            fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
                match d.datatype()? {
                    $( minicbor::data::Type::Array => {
                        d.array()?;

                        Ok($enum_name::$many_f($( d.decode_with::<_, $types>(ctx)?, )+ ))
                    }, )?
                    $( $( minicbor::data::Type::$cbortype )|* => Ok($enum_name::$one_f(d.decode_with(ctx)?)), )*
                    _ => Err(minicbor::decode::Error::message(
                            "Unknown cbor data type for this macro-defined enum.")
                    ),
                }
            }
        }

        impl<C> minicbor::encode::Encode<C> for $enum_name {
            fn encode<W: minicbor::encode::Write>(
                &self,
                e: &mut minicbor::Encoder<W>,
                ctx: &mut C,
            ) -> Result<(), minicbor::encode::Error<W::Error>> {
                match self {
                    $( $enum_name::$many_f ($( $vars ),+) => {
                        e.array(2)?;
                        $( e.encode_with($vars, ctx)?; )+
                    }, )?
                    $( $enum_name::$one_f(__x666) => {
                        e.encode_with(__x666, ctx)?;
                    } )*
                };

                Ok(())
            }
        }
    }
}
