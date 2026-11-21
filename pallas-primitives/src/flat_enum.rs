//! Decoding of enums written as an array whose first element is the variant index.

#[cfg(test)]
use pallas_codec::minicbor;

/// Implements `Decode` for an enum from a table of each variant's index and fields, reading an array of either length form.
macro_rules! decode_flat_enum {
    (@field $a:ident, $ctx:ident, $field:ident) => {
        $a.decode_with($ctx)?
    };
    (@field $a:ident, $ctx:ident, $field:ident, $by:path) => {
        $a.decode_by($ctx, $by)?
    };
    (
        $name:ident $(<$lt:lifetime>)?,
        $(
            $index:literal => $variant:ident
            $( ( $( $field:ident $(: $by:path)? ),+ ) )?
            $( { $( $named:ident $(: $named_by:path)? ),+ } )?
        ),+ $(,)?
    ) => {
        impl<'bytes $(: $lt, $lt)?, Ctx> pallas_codec::minicbor::Decode<'bytes, Ctx>
            for $name $(<$lt>)?
        {
            #[deny(unreachable_patterns)]
            fn decode(
                d: &mut pallas_codec::minicbor::Decoder<'bytes>,
                ctx: &mut Ctx,
            ) -> Result<$name $(<$lt>)?, pallas_codec::minicbor::decode::Error> {
                let start = d.position();
                pallas_codec::utils::decode_array(d, |a| {
                    let index: i64 = a.decode_with(ctx)?;
                    match index {
                        $(
                            $index => Ok($name::$variant
                                $( ( $( $crate::flat_enum::decode_flat_enum!(@field a, ctx, $field $(, $by)?) ),+ ) )?
                                $( { $( $named: $crate::flat_enum::decode_flat_enum!(@field a, ctx, $named $(, $named_by)?) ),+ } )?
                            ),
                        )+
                        n => Err(pallas_codec::minicbor::decode::Error::unknown_variant(n).at(start)),
                    }
                })
            }
        }

        const _: () = {
            #[allow(dead_code)]
            fn every_variant_has_an_index $(<$lt>)? (value: &$name $(<$lt>)?) {
                match value {
                    $( $name::$variant { .. } => {} )+
                }
            }
        };
    };
}

pub(crate) use decode_flat_enum;

/// Decodes `bytes` as one `T` that ends exactly where the bytes do.
#[cfg(test)]
pub(crate) fn decode_all<'b, T: minicbor::Decode<'b, ()>>(bytes: &'b [u8]) -> T {
    let mut d = minicbor::Decoder::new(bytes);
    let value = d
        .decode()
        .unwrap_or_else(|e| panic!("{} must decode: {e}", hex::encode(bytes)));
    assert_eq!(
        d.position(),
        bytes.len(),
        "{} must be read to its last byte",
        hex::encode(bytes)
    );
    value
}

/// Asserts that each definite array encoding decodes to a value that re-encodes to the same bytes, and that the same elements in an indefinite array decode to that value.
#[cfg(test)]
pub(crate) fn assert_reads_either_length_form<T>(encodings: &[String])
where
    T: minicbor::Decode<'static, ()> + minicbor::Encode<()> + PartialEq + std::fmt::Debug,
{
    for encoding in encodings {
        let definite: &'static [u8] = Box::leak(hex::decode(encoding).unwrap().into_boxed_slice());
        let [header, elements @ ..] = definite else {
            panic!("{encoding} is empty");
        };
        assert!(
            (0x81..=0x97).contains(header),
            "{encoding} must start with a definite array header of one byte"
        );
        let indefinite: &'static [u8] =
            Box::leak([&[0x9f], elements, &[0xff]].concat().into_boxed_slice());

        let from_definite: T = decode_all(definite);
        assert_eq!(
            minicbor::to_vec(&from_definite).unwrap(),
            definite,
            "{encoding} must re-encode to its own bytes"
        );
        let from_indefinite: T = decode_all(indefinite);
        assert_eq!(
            from_indefinite, from_definite,
            "{encoding} must read the same with an indefinite array"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::assert_reads_either_length_form;
    use crate::StakeCredential;
    use pallas_codec::minicbor;

    const HASH: &str = "581c11111111111111111111111111111111111111111111111111111111";

    #[test]
    fn a_credential_reads_either_length_form() {
        assert_reads_either_length_form::<StakeCredential>(&[
            format!("8200{HASH}"),
            format!("8201{HASH}"),
        ]);
    }

    #[test]
    fn a_credential_with_extra_elements_reads_to_the_end_of_its_array() {
        for bytes in [format!("8300{HASH}00"), format!("9f00{HASH}9f00ff00ff")] {
            let bytes = hex::decode(&bytes).unwrap();
            let mut d = minicbor::Decoder::new(&bytes);
            let credential: StakeCredential = d.decode().unwrap();
            assert!(matches!(credential, StakeCredential::AddrKeyhash(_)));
            assert_eq!(d.position(), bytes.len());
        }
    }

    #[test]
    fn a_credential_without_its_hash_or_with_an_unknown_index_is_refused() {
        for bytes in [
            "8100".to_string(),
            "9f00ff".to_string(),
            format!("8202{HASH}"),
            format!("9f02{HASH}ff"),
        ] {
            assert!(
                minicbor::decode::<StakeCredential>(&hex::decode(&bytes).unwrap()).is_err(),
                "{bytes} must be refused"
            );
        }
    }
}
