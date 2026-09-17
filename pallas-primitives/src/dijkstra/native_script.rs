//! The Shelley native script plus this era's guard clause. Everything about
//! its lifecycle lives in [`crate::native_script`].

use super::NativeScript;
use crate::{StakeCredential, native_script::impl_native_script};

impl_native_script!(NativeScript { 6 => ScriptRequireGuard(StakeCredential) });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StakeCredential;
    use pallas_codec::minicbor::{self, Encoder};
    use proptest::prelude::*;

    fn credentials() -> impl Strategy<Value = StakeCredential> {
        prop_oneof![
            any::<[u8; 28]>().prop_map(|x| StakeCredential::AddrKeyhash(x.into())),
            any::<[u8; 28]>().prop_map(|x| StakeCredential::ScriptHash(x.into())),
        ]
    }

    fn scripts() -> impl Strategy<Value = NativeScript> {
        prop_oneof![
            any::<[u8; 28]>().prop_map(|x| NativeScript::ScriptPubkey(x.into())),
            any::<u64>().prop_map(NativeScript::InvalidBefore),
            any::<u64>().prop_map(NativeScript::InvalidHereafter),
            credentials().prop_map(NativeScript::ScriptRequireGuard),
        ]
        .prop_recursive(4, 64, 8, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..5).prop_map(NativeScript::ScriptAll),
                prop::collection::vec(inner.clone(), 0..5).prop_map(NativeScript::ScriptAny),
                (any::<i64>(), prop::collection::vec(inner, 0..5))
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
                    NativeScript::ScriptRequireGuard(StakeCredential::ScriptHash(key.into())),
                ]),
                format!("8201828204018206 8201581c{key_hex}").replace(' ', ""),
            ),
            (NativeScript::ScriptAny(vec![]), "820280".to_string()),
            (
                NativeScript::ScriptNOfK(
                    2,
                    vec![
                        NativeScript::ScriptRequireGuard(StakeCredential::AddrKeyhash(key.into())),
                        NativeScript::ScriptAny(vec![NativeScript::InvalidBefore(0)]),
                    ],
                ),
                format!("83030282 8206 8200581c{key_hex} 820281820400").replace(' ', ""),
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

    /// The six variants shared with Alonzo encode identically in both eras.
    #[test]
    fn shares_alonzo_wire_format_for_common_variants() {
        use crate::alonzo::NativeScript as Alonzo;

        let key = [0x11; 28];
        let pairs = [
            (
                NativeScript::ScriptPubkey(key.into()),
                Alonzo::ScriptPubkey(key.into()),
            ),
            (
                NativeScript::ScriptAll(vec![NativeScript::InvalidBefore(7)]),
                Alonzo::ScriptAll(vec![Alonzo::InvalidBefore(7)]),
            ),
            (
                NativeScript::ScriptAny(vec![NativeScript::InvalidHereafter(9)]),
                Alonzo::ScriptAny(vec![Alonzo::InvalidHereafter(9)]),
            ),
            (
                NativeScript::ScriptNOfK(-1, vec![]),
                Alonzo::ScriptNOfK(-1, vec![]),
            ),
        ];
        for (dijkstra, alonzo) in pairs {
            let bytes = minicbor::to_vec(&dijkstra).unwrap();
            assert_eq!(bytes, minicbor::to_vec(&alonzo).unwrap());
            let back: NativeScript = minicbor::decode(&bytes).unwrap();
            assert!(back == dijkstra);
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
        // The leaf is this era's guard clause.
        bytes.extend_from_slice(&[0x82, 6, 0x82, 0, 0x58, 0x1c]);
        bytes.extend_from_slice(&[0x7a; 28]);
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
            let mut bytes = vec![0x82, 1, 0x82];
            bytes.extend(deep_cbor(20_000));
            bytes.extend_from_slice(&[0x82, 0, 0x40]); // pubkey of incorrect size
            assert!(minicbor::decode::<NativeScript>(&bytes).is_err());

            let mut bytes = deep_cbor(20_000);
            bytes.pop();
            assert!(minicbor::decode::<NativeScript>(&bytes).is_err());
        });
    }

    #[test]
    fn malformed_arrays_are_rejected() {
        for bytes in [
            "80",
            "8100",
            "820301",
            "9f0400ff",
            "820780",                 // unknown variant, one past the guard clause
            "822080",                 // negative variant
            "82019bffffffffffffffff", // enormous truncated child list
            "8206820258",             // truncated guard credential
        ] {
            let bytes = hex::decode(bytes).unwrap();
            assert!(
                minicbor::decode::<NativeScript>(&bytes).is_err(),
                "{bytes:02x?}"
            );
        }
    }

    #[test]
    fn equality_checks_variant_payload_order_and_arity() {
        let key = [0; 28];
        let guard = NativeScript::ScriptRequireGuard(StakeCredential::AddrKeyhash(key.into()));
        let other_guard = NativeScript::ScriptRequireGuard(StakeCredential::ScriptHash(key.into()));
        assert!(guard != other_guard);
        assert!(guard != NativeScript::ScriptPubkey(key.into()));
        assert!(NativeScript::ScriptAll(vec![guard.clone()]) != NativeScript::ScriptAll(vec![]));
        assert!(
            NativeScript::ScriptAll(vec![guard.clone(), other_guard.clone()])
                != NativeScript::ScriptAll(vec![other_guard, guard])
        );
    }
}
