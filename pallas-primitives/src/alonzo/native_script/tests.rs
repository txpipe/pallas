use super::*;
use proptest::prelude::*;

// Keep the old derived codec as an oracle for shallow trees and wire
// compatibility, including permissive handling of extra variant fields.
#[derive(Debug, Encode, Decode)]
#[cbor(flat)]
enum LegacyScript {
    #[n(0)]
    Pubkey(#[n(0)] crate::AddrKeyhash),
    #[n(1)]
    All(#[n(0)] Vec<LegacyScript>),
    #[n(2)]
    Any(#[n(0)] Vec<LegacyScript>),
    #[n(3)]
    NOfK(#[n(0)] u32, #[n(1)] Vec<LegacyScript>),
    #[n(4)]
    Before(#[n(0)] u64),
    #[n(5)]
    After(#[n(0)] u64),
}

fn legacy_scripts() -> impl Strategy<Value = LegacyScript> {
    prop_oneof![
        any::<[u8; 28]>().prop_map(|x| LegacyScript::Pubkey(x.into())),
        any::<u64>().prop_map(LegacyScript::Before),
        any::<u64>().prop_map(LegacyScript::After),
    ]
    .prop_recursive(4, 64, 8, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..5).prop_map(LegacyScript::All),
            prop::collection::vec(inner.clone(), 0..5).prop_map(LegacyScript::Any),
            (any::<u32>(), prop::collection::vec(inner, 0..5))
                .prop_map(|(n, xs)| LegacyScript::NOfK(n, xs)),
        ]
    })
}

proptest! {
    #[test]
    fn matches_derived_codec(legacy in legacy_scripts()) {
        let bytes = minicbor::to_vec(&legacy).unwrap();
        let script: NativeScript = minicbor::decode(&bytes).unwrap();
        prop_assert_eq!(minicbor::to_vec(&script).unwrap(), bytes);
        prop_assert!(script == script.clone());
    }
}

fn deep_cbor(depth: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    for i in 0..depth {
        match i % 3 {
            0 => bytes.extend_from_slice(&[0x82, 1, 0x81]),
            1 => bytes.extend_from_slice(&[0x82, 2, 0x81]),
            _ => bytes.extend_from_slice(&[0x83, 3, 1, 0x81]),
        }
    }
    bytes.extend_from_slice(&[0x82, 4, 0]);
    bytes
}

fn small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deep_script_lifecycle_on_small_stack() {
    small_stack(|| {
        let bytes = deep_cbor(20_000);
        let script: NativeScript = minicbor::decode(&bytes).unwrap();
        let cloned = script.clone();
        assert!(script == cloned);
        assert_eq!(minicbor::to_vec(&cloned).unwrap(), bytes);
        let mut different_bytes = bytes.clone();
        *different_bytes.last_mut().unwrap() = 1;
        let different: NativeScript = minicbor::decode(&different_bytes).unwrap();
        assert!(script != different);

        let mut short_buffer = [0u8; 16];
        assert!(
            Encoder::new(short_buffer.as_mut_slice())
                .encode(&script)
                .is_err()
        );
        // All three trees are dropped on this same small stack.
    });
}

#[test]
fn malformed_deep_script_cleans_up_completed_children() {
    small_stack(|| {
        // An outer All has a fully decoded deep child followed by a
        // malformed sibling. Error cleanup must drop that completed tree.
        let mut bytes = vec![0x82, 1, 0x82];
        bytes.extend(deep_cbor(20_000));
        bytes.extend_from_slice(&[0x82, 0, 0x40]); // pubkey of incorrect size
        assert!(minicbor::decode::<NativeScript>(&bytes).is_err());

        // Also exercise cleanup of thousands of incomplete parent frames.
        let mut bytes = deep_cbor(20_000);
        bytes.pop();
        assert!(minicbor::decode::<NativeScript>(&bytes).is_err());
    });
}

#[test]
fn preserves_permissive_array_decoding() {
    for bytes in [
        "820180",               // empty All
        "82029fff",             // empty indefinite Any
        "83030080",             // empty NOfK
        "82019f820400820501ff", // indefinite child list
        "830400f6",             // trailing field on a leaf
        "830181830501f68100",   // trailing fields on a parent and its child
    ] {
        let bytes = hex::decode(bytes).unwrap();
        let legacy: LegacyScript = minicbor::decode(&bytes).unwrap();
        let mut d = Decoder::new(&bytes);
        let script: NativeScript = d.decode().unwrap();
        assert_eq!(d.position(), bytes.len());
        assert_eq!(
            minicbor::to_vec(&script).unwrap(),
            minicbor::to_vec(legacy).unwrap()
        );
    }
}

#[test]
fn malformed_arrays_are_rejected_without_reserving_claimed_length() {
    for bytes in [
        "80",
        "8100",
        "820301",
        "9f0400ff", // missing fields / indefinite variant
        "820680",
        "822080",                 // unknown positive / negative variant
        "82019bffffffffffffffff", // enormous truncated child list
        "82019f820400",           // missing indefinite child-list break
        "830181820400",           // missing trailing field after a completed child
    ] {
        let bytes = hex::decode(bytes).unwrap();
        assert!(minicbor::decode::<LegacyScript>(&bytes).is_err());
        assert!(minicbor::decode::<NativeScript>(&bytes).is_err());
    }
}

#[test]
fn equality_checks_variant_payload_order_and_arity() {
    let a = NativeScript::InvalidBefore(1);
    let b = NativeScript::InvalidHereafter(1);
    assert!(a != b);
    assert!(NativeScript::ScriptAll(vec![]) != NativeScript::ScriptAny(vec![]));
    assert!(NativeScript::ScriptNOfK(1, vec![]) != NativeScript::ScriptNOfK(2, vec![]));
    assert!(
        NativeScript::ScriptAll(vec![a.clone(), b.clone()])
            != NativeScript::ScriptAll(vec![b, a.clone()])
    );
    assert!(NativeScript::ScriptAll(vec![a]) != NativeScript::ScriptAll(vec![]));
    assert!(
        NativeScript::ScriptPubkey([0; 28].into()) != NativeScript::ScriptPubkey([1; 28].into())
    );
}
