/// Flat encoding/decoding for Plutus Core
pub mod flat;

/// Shared re-export of minicbor lib across all Pallas
pub use minicbor;

/// Round-trip friendly common helper structs
pub mod utils;

/// Heap to track the original cbor bytes for decoded structs
pub mod cborheap;

pub trait Fragment: Sized + for<'b> minicbor::Decode<'b, ()> + minicbor::Encode<()> {}

impl<T> Fragment for T where T: for<'b> minicbor::Decode<'b, ()> + minicbor::Encode<()> + Sized {}

#[macro_export]
macro_rules! codec_by_datatype {
    (
        $enum_name:ident $( < $lifetime:lifetime > )?,
        $( $( $cbortype:ident )|* => $one_f:ident ),*,
        ($( $( $vars:ident ),+ => $many_f:ident )?)
    ) => {
        impl<$( $lifetime, )? '__b $(:$lifetime)?,  C> minicbor::decode::Decode<'__b, C> for $enum_name $(<$lifetime>)? {
            fn decode(d: &mut minicbor::Decoder<'__b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
                match d.datatype()? {
                    $( minicbor::data::Type::Array => {
                        d.array()?;
                        // Using the identifiers trivially to ensure repetition.
                        Ok($enum_name::$many_f($({ let $vars = d.decode_with(ctx)?; $vars }, )+ ))
                    }, )?
                    $( $( minicbor::data::Type::$cbortype )|* => Ok($enum_name::$one_f(d.decode_with(ctx)?)), )*
                    _ => Err(minicbor::decode::Error::message(
                            "Unknown cbor data type for this macro-defined enum.")
                    ),
                }
            }
        }

        impl< $( $lifetime, )? C> minicbor::encode::Encode<C> for $enum_name $(<$lifetime>)?  {
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
