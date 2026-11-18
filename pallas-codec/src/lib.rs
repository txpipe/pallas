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
//! - [`tree`] — stack-safe decoding and traversal of recursive types via
//!   [`tree::TreeDecode`], [`tree::decode_tree`] and [`tree::TreeNode`];
//!   [`tree::decode_tree_with_max_depth`] bounds the nesting depth.
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

/// Stack-safe decoding of recursive CBOR structures.
pub mod tree;

pub use tree::MAX_DECODE_DEPTH;

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
/// to a single-payload variant, and an array of either length form handles a
/// many-field variant.
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
                    $( minicbor::data::Type::Array | minicbor::data::Type::ArrayIndef => {
                        let len = d.array()?;
                        let mut read = 0;
                        // Using the identifiers trivially to ensure repetition.
                        let value = $enum_name::$many_f($({ let $vars = d.decode_with(ctx)?; read += 1; $vars }, )+ );
                        $crate::utils::skip_array_rest(d, len, read)?;
                        Ok(value)
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
    use super::minicbor::{self, Decode, Encode, decode, encode};

    #[derive(Clone, Debug, PartialEq)]
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

    /// Decodes `bytes` as one `Thing` and the position the decoder stopped at.
    fn decode_thing(bytes: &[u8]) -> Result<(Thing, usize), decode::Error> {
        let mut d = minicbor::Decoder::new(bytes);
        let thing = d.decode()?;
        Ok((thing, d.position()))
    }

    #[test]
    fn a_many_field_variant_reads_any_array_of_its_fields_to_the_end() {
        let cases: [(&str, &[u8]); 3] = [
            ("definite", &[0x83, 0xf5, 0x0a, 0x33]),
            ("indefinite", &[0x9f, 0xf5, 0x0a, 0x33, 0xff]),
            (
                "definite with an extra element",
                &[0x84, 0xf5, 0x0a, 0x33, 0x00],
            ),
        ];
        for (name, bytes) in cases {
            let (thing, position) =
                decode_thing(bytes).unwrap_or_else(|e| panic!("the {name} array: {e}"));
            assert_eq!(thing, Thing::Multiasset(true, 10, -20), "the {name} array");
            assert_eq!(
                position,
                bytes.len(),
                "the {name} array ends at its last byte"
            );
        }
    }

    #[test]
    fn a_many_field_variant_reads_its_fields_past_the_end_of_a_short_definite_array() {
        let (thing, position) = decode_thing(&[0x82, 0xf5, 0x0a, 0x00]).unwrap();
        assert_eq!(thing, Thing::Multiasset(true, 10, 0));
        assert_eq!(position, 4);
    }

    #[test]
    fn a_many_field_variant_refuses_an_indefinite_array_short_of_its_fields() {
        assert!(decode_thing(&[0x9f, 0xf5, 0x0a, 0xff]).is_err());
    }
}
