//! Deep trees must decode and clone on a small stack. `Drop` still recurses,
//! so every deep value is leaked.
use pallas_codec::minicbor::{self, Decode};
use pallas_primitives::{Metadatum, PlutusData};
use std::mem::ManuallyDrop;

/// Nesting below the decode depth cap, with room for the two nodes the wide
/// case wraps it in.
const DEEP: usize = pallas_codec::MAX_DECODE_DEPTH - 3;

fn nested(prefix: &[u8], suffix: &[u8]) -> Vec<u8> {
    let mut bytes = prefix.repeat(DEEP);
    bytes.push(0);
    bytes.extend(suffix.repeat(DEEP));
    bytes
}

fn lifecycle<T>(bytes: &[u8])
where
    T: for<'b> Decode<'b, ()> + Clone,
{
    let tree = ManuallyDrop::new(minicbor::decode::<T>(bytes).unwrap());
    let _copy = ManuallyDrop::new(T::clone(&tree));
}

#[test]
fn deep_trees_decode_and_clone_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            for (prefix, suffix) in [
                (&[0x81][..], &[][..]),
                (&[0x9f][..], &[0xff][..]),
                (&[0xa1, 0][..], &[][..]),
                (&[0xbf, 0][..], &[0xff][..]),
                (&[0xa1][..], &[0][..]),
                (&[0xbf][..], &[0, 0xff][..]),
            ] {
                let bytes = nested(prefix, suffix);
                lifecycle::<Metadatum>(&bytes);
                lifecycle::<PlutusData>(&bytes);
            }
            for prefix in [&[0xd8, 0x79, 0x81][..], &[0xd8, 0x66, 0x82, 0, 0x81][..]] {
                lifecycle::<PlutusData>(&nested(prefix, &[]));
            }

            // Mixed nodes and siblings exercise pending work on both sides
            // of a map, rather than only a single-child chain.
            let mut wide = vec![0xa2];
            wide.extend(nested(&[0x81], &[]));
            wide.extend(nested(&[0xa1, 0], &[]));
            wide.extend([0, 0x82]);
            wide.extend(nested(&[0x9f], &[0xff]));
            wide.push(0);
            lifecycle::<Metadatum>(&wide);
            lifecycle::<PlutusData>(&wide);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn consuming_accessors_preserve_payloads_and_mismatched_values() {
    macro_rules! check {
        ($ty:ty, $variant:ident, $method:ident, $hex:expr) => {{
            let bytes = hex::decode($hex).unwrap();
            let value: $ty = minicbor::decode(&bytes).unwrap();
            let payload = value.$method().unwrap();
            assert_eq!(minicbor::to_vec(<$ty>::$variant(payload)).unwrap(), bytes);
            let other: $ty = minicbor::decode(&[0x00]).unwrap();
            assert!(other.$method().is_err());
        }};
    }
    check!(Metadatum, Bytes, into_bytes, "4100");
    check!(Metadatum, Text, into_text, "6161");
    check!(Metadatum, Array, into_array, "8100");
    check!(Metadatum, Map, into_map, "bf0001ff");
    check!(PlutusData, Constr, into_constr, "d8798100");
    check!(PlutusData, Map, into_map, "bf0001ff");
    check!(PlutusData, Array, into_array, "9f00ff");
    check!(PlutusData, BoundedBytes, into_bounded_bytes, "4100");
    let value: PlutusData = minicbor::decode(&[0x00]).unwrap();
    assert_eq!(
        minicbor::to_vec(PlutusData::BigInt(value.into_bigint().unwrap())).unwrap(),
        [0]
    );
    let other: PlutusData = minicbor::decode(&[0x80]).unwrap();
    assert!(matches!(other.into_bigint(), Err(PlutusData::Array(_))));
}
