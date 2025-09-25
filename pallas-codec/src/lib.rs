/// Flat encoding/decoding for Plutus Core
pub mod flat;

/// Shared re-export of minicbor lib across all Pallas
pub use minicbor;

/// Round-trip friendly common helper structs
pub mod utils;

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
                        // Counting the number of `$vars`:
                        let length: u64 = 0 $(+ { let _ = $vars; 1 })+;
                        e.array(length)?;
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

#[cfg(test)]
mod tests {
    use super::minicbor::{self, decode, encode, Decode, Encode};

    #[derive(Clone, Debug)]
    enum Thing {
        Coin(u32),
        Change(bool),
        Multiasset(bool, u64, i32),
    }

    codec_by_datatype! {
        Thing,
        U8 | U16 | U32 => Coin,
        Bool => Change,
        (b, u, i => Multiasset)
    }

    #[cfg(test)]
    pub fn roundtrip_codec<T: Encode<()> + for<'a> Decode<'a, ()> + std::fmt::Debug>(
        query: T,
    ) -> () {
        let mut cbor = Vec::new();
        match encode(query, &mut cbor) {
            Ok(_) => (),
            Err(err) => panic!("Unable to encode data ({:?})", err),
        };
        println!("{:-<70}\nResulting CBOR: {:02x?}", "", cbor);

        let query: T = decode(&cbor).unwrap();
        println!("Decoded data: {:?}", query);
    }

    #[test]
    fn roundtrip_codec_by_datatype() {
        roundtrip_codec(Thing::Coin(0xfafa));
        roundtrip_codec(Thing::Change(false));
        roundtrip_codec(Thing::Multiasset(true, 10, -20));
    }
}
