use super::*;
use proptest::prelude::*;

fn scripts() -> impl Strategy<Value = NativeScript> {
    prop_oneof![
        any::<[u8; 28]>().prop_map(|x| NativeScript::ScriptPubkey(x.into())),
        any::<u64>().prop_map(NativeScript::InvalidBefore),
        any::<u64>().prop_map(NativeScript::InvalidHereafter),
    ]
    .prop_recursive(4, 64, 8, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..5).prop_map(NativeScript::ScriptAll),
            prop::collection::vec(inner.clone(), 0..5).prop_map(NativeScript::ScriptAny),
            (
                prop_oneof![
                    Just(i64::MIN),
                    Just(-1i64),
                    Just(0i64),
                    Just(i64::MAX),
                    any::<i64>(),
                ],
                prop::collection::vec(inner, 0..5),
            )
                .prop_map(|(n, xs)| NativeScript::ScriptNOfK(n, xs)),
        ]
    })
}

proptest! {
    #[test]
    fn round_trips_through_cbor(script in scripts()) {
        let bytes = minicbor::to_vec(&script).unwrap();
        let decoded: NativeScript = minicbor::decode(&bytes).unwrap();
        prop_assert!(decoded == script);
        prop_assert_eq!(minicbor::to_vec(&decoded).unwrap(), bytes);
        prop_assert!(script.clone() == script);
    }
}

#[test]
fn matches_ledger_cddl_wire_format() {
    let key = [0xab; 28];
    let key_hex = "ab".repeat(28);
    let cases = [
        (
            NativeScript::ScriptPubkey(key.into()),
            format!("8200581c{key_hex}"),
        ),
        (
            NativeScript::ScriptAll(vec![
                NativeScript::InvalidBefore(1),
                NativeScript::InvalidHereafter(2),
            ]),
            "820182820401820502".to_string(),
        ),
        (NativeScript::ScriptAny(vec![]), "820280".to_string()),
        (
            NativeScript::ScriptNOfK(
                2,
                vec![
                    NativeScript::ScriptPubkey(key.into()),
                    NativeScript::ScriptAny(vec![NativeScript::InvalidBefore(0)]),
                ],
            ),
            format!("830302828200581c{key_hex}820281820400"),
        ),
        (NativeScript::InvalidBefore(1000), "82041903e8".to_string()),
        (
            NativeScript::InvalidHereafter(1 << 32),
            "82051b0000000100000000".to_string(),
        ),
    ];
    for (script, hex) in cases {
        let bytes = hex::decode(&hex).unwrap();
        assert_eq!(minicbor::to_vec(&script).unwrap(), bytes, "{hex}");
        let decoded: NativeScript = minicbor::decode(&bytes).unwrap();
        assert!(decoded == script, "{hex}");
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
    use NativeScript::*;
    let cases = [
        ("820180", ScriptAll(vec![]), "820180"),
        ("82029fff", ScriptAny(vec![]), "820280"),
        ("83030080", ScriptNOfK(0, vec![]), "83030080"),
        (
            "82019f820400820501ff",
            ScriptAll(vec![InvalidBefore(0), InvalidHereafter(1)]),
            "820182820400820501",
        ),
        ("830400f6", InvalidBefore(0), "820400"),
        (
            "830181830501f68100",
            ScriptAll(vec![InvalidHereafter(1)]),
            "820181820501",
        ),
    ];
    for (input, expected, canonical) in cases {
        let bytes = hex::decode(input).unwrap();
        let mut d = Decoder::new(&bytes);
        let script: NativeScript = d.decode().unwrap();
        assert_eq!(d.position(), bytes.len(), "{input}");
        assert!(script == expected, "{input}");
        assert_eq!(
            minicbor::to_vec(&script).unwrap(),
            hex::decode(canonical).unwrap(),
            "{input}"
        );
    }
}

#[test]
fn n_of_k_threshold_is_signed() {
    // Preprod tx b1db2a411cb651a413840d3c8b112895a5bda2519a8ba6a372b8dd1ffc7746c2
    // carries exactly this script, with a threshold of -1.
    let bytes = hex::decode(
        "830320828200581c3118644aa21ba172c82732ce80d1c94cdcb5f2e8891e1ad2645707188200581ce07caf4bf751495f75774ace30552441e4df84d141e5d1f5029cb04d",
    )
    .unwrap();
    let script: NativeScript = minicbor::decode(&bytes).unwrap();
    let NativeScript::ScriptNOfK(n, scripts) = &script else {
        panic!("expected ScriptNOfK, got {script:?}");
    };
    assert_eq!(*n, -1);
    assert_eq!(scripts.len(), 2);
    assert_eq!(minicbor::to_vec(&script).unwrap(), bytes);
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
