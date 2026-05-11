//! The encoding foundation the rest of the Pallas workspace builds on.
//!
//! Provides [`minicbor`] for CBOR (re-exported as-is) and a Rust port of the
//! Plutus Core [flat] format. Most users won't depend on this crate directly
//! — they'll get its types transitively through `pallas-primitives`,
//! `pallas-traverse`, `pallas-txbuilder`, and so on. Reach for it when you
//! need to define your own minicbor-encoded type, or when you need the
//! round-trip helpers ([`utils::KeepRaw`], [`utils::Nullable`],
//! [`utils::Set`], …) used by the higher-level era types.
//!
//! [flat]: https://github.com/Quid2/flat
//!
//! # Usage
//!
//! ```
//! use pallas_codec::minicbor;
//!
//! #[derive(minicbor::Encode, minicbor::Decode, Debug, PartialEq)]
//! struct Pair(#[n(0)] u64, #[n(1)] String);
//!
//! let bytes = minicbor::to_vec(Pair(1, "hi".into()))?;
//! let back: Pair = minicbor::decode(&bytes)?;
//! assert_eq!(back, Pair(1, "hi".into()));
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
//!
//! # Overview
//!
//! - [`minicbor`] — re-exported as-is; this is the workspace's single source
//!   of truth for CBOR.
//! - [`flat`] — Rust port of the Haskell [flat] reference implementation,
//!   used for Plutus Core scripts.
//! - [`utils`] — round-trip-friendly helper types ([`utils::KeepRaw`],
//!   [`utils::KeyValuePairs`], [`utils::MaybeIndefArray`],
//!   [`utils::NonEmptySet`], [`utils::Nullable`], [`utils::PositiveCoin`],
//!   …) reused by the higher-level era types.
//! - [`Fragment`] trait — blanket-implemented for any type that is both
//!   [`minicbor::Encode`] and [`minicbor::Decode`]; used as a bound where
//!   the workspace wants "any CBOR-roundtrippable type".
//! - [`codec_by_datatype!`] macro — derives a tag-free CBOR codec for enums
//!   whose variants are distinguished by their data-type rather than a
//!   discriminant.
//!
//! # Usage as part of `pallas`
//!
//! When depending on the umbrella [`pallas`] crate, this crate is re-exported
//! as `pallas::codec`.
//!
//! [`pallas`]: https://crates.io/crates/pallas

/// Flat encoding/decoding for Plutus Core.
pub mod flat;

/// Shared re-export of `minicbor` across all Pallas crates.
pub use minicbor;

/// Round-trip friendly common helper structs (`Bytes`, `Nullable`, `Set`, …).
pub mod utils;

/// Blanket trait for any type that can be CBOR-encoded and decoded with
/// [`minicbor`]. Implemented automatically for every such type.
pub trait Fragment: Sized + for<'b> minicbor::Decode<'b, ()> + minicbor::Encode<()> {}

impl<T> Fragment for T where T: for<'b> minicbor::Decode<'b, ()> + minicbor::Encode<()> + Sized {}

/// Derive a `minicbor` [`Decode`]/[`Encode`] implementation for an enum by
/// dispatching on the incoming CBOR datatype.
///
/// Useful for sum types whose variants carry distinct CBOR shapes (e.g. one
/// variant is an array, another is a map). The macro maps each CBOR datatype
/// to a single-payload variant, and an `Array` fallback handles a many-field
/// variant.
///
/// [`Decode`]: minicbor::Decode
/// [`Encode`]: minicbor::Encode
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
    pub fn roundtrip_codec<T: Encode<()> + for<'a> Decode<'a, ()> + std::fmt::Debug>(query: T) {
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
