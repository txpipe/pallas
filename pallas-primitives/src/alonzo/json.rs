use std::ops::Deref;

use pallas_codec::tree::{Visit, map_tree, walk_tree};
use serde_json::json;

use crate::ToCanonicalJson;

impl<A> super::Constr<A> {
    pub fn constructor_value(&self) -> Option<u64> {
        match self.tag {
            121..=127 => Some(self.tag - 121),
            1280..=1400 => Some(self.tag - 1280 + 7),
            102 => self.any_constructor,
            _ => None,
        }
    }
}

// infered from https://github.com/input-output-hk/cardano-node/blob/c1efb2f97134c0607c982246a36e3da7266ac194/cardano-api/src/Cardano/Api/ScriptData.hs#L254
impl ToCanonicalJson for super::PlutusData {
    fn to_json(&self) -> serde_json::Value {
        match self {
            super::PlutusData::Constr(x) => {
                let fields: Vec<_> = x.fields.iter().map(|i| i.to_json()).collect();
                json!({ "constructor": x.constructor_value(), "fields": fields })
            }
            super::PlutusData::Map(x) => {
                let map: Vec<_> = x
                    .iter()
                    .map(|(k, v)| json!({ "k": k.to_json(), "v": v.to_json() }))
                    .collect();
                json!({ "map": map })
            }
            super::PlutusData::BigInt(int) => match int {
                super::BigInt::Int(n) => match i64::try_from(*n.deref()) {
                    Ok(x) => json!({ "int": x }),
                    Err(_) => {
                        json!({ "bignint": hex::encode(i128::from(*n.deref()).to_be_bytes()) })
                    }
                },
                // WARNING / TODO: the CDDL shows a bignum variants of arbitrary length expressed as
                // bytes, but I can't find the corresponding mapping to JSON in the
                // Haskell implementation. Not sure what I'm missing. For the time
                // being, I'll invent a new JSON expression that uses hex strings as
                // a way to express the values.
                super::BigInt::BigUInt(x) => json!({ "biguint": hex::encode(x.as_slice())}),
                super::BigInt::BigNInt(x) => json!({ "bignint": hex::encode(x.as_slice())}),
            },
            super::PlutusData::BoundedBytes(x) => json!({ "bytes": hex::encode(x.as_slice())}),
            super::PlutusData::Array(x) => {
                let list: Vec<_> = x.iter().map(|i| i.to_json()).collect();
                json!({ "list": list })
            }
        }
    }
}

impl ToCanonicalJson for super::NativeScript {
    fn to_json(&self) -> serde_json::Value {
        use super::NativeScript;

        fn shallow(x: &NativeScript) -> serde_json::Value {
            match x {
                NativeScript::ScriptPubkey(x) => json!({ "keyHash": x.to_string(), "type": "sig"}),
                NativeScript::ScriptAll(_) => json!({ "type": "all", "scripts": []}),
                NativeScript::ScriptAny(_) => json!({ "type": "any", "scripts": []}),
                NativeScript::ScriptNOfK(n, _) => {
                    json!({ "type": "atLeast", "required": n, "scripts": []})
                }
                NativeScript::InvalidBefore(slot) => json!({ "type": "after", "slot": slot }),
                NativeScript::InvalidHereafter(slot) => json!({"type": "before", "slot": slot }),
            }
        }

        map_tree(self, shallow, |value: &mut serde_json::Value| {
            value
                .get_mut("scripts")
                .and_then(serde_json::Value::as_array_mut)
        })
    }

    /// Writes the JSON text directly, never building a `serde_json::Value`:
    /// that tree drops, clones and compares recursively, and a chain-deep
    /// script overflows a 2 MiB stack doing so.
    fn to_json_string(&self) -> String {
        use std::fmt::Write;

        use super::NativeScript;

        let mut out = String::new();
        walk_tree::<_, std::fmt::Error>(self, |visit| match visit {
            Visit::Enter(NativeScript::ScriptPubkey(x)) => {
                write!(out, r#"{{"keyHash":"{x}","type":"sig"}}"#)
            }
            Visit::Enter(NativeScript::ScriptAll(_) | NativeScript::ScriptAny(_)) => {
                write!(out, r#"{{"scripts":["#)
            }
            Visit::Enter(NativeScript::ScriptNOfK(n, _)) => {
                write!(out, r#"{{"required":{n},"scripts":["#)
            }
            Visit::Enter(NativeScript::InvalidBefore(slot)) => {
                write!(out, r#"{{"slot":{slot},"type":"after"}}"#)
            }
            Visit::Enter(NativeScript::InvalidHereafter(slot)) => {
                write!(out, r#"{{"slot":{slot},"type":"before"}}"#)
            }
            Visit::Between(_) => write!(out, ","),
            Visit::Exit(NativeScript::ScriptAll(_)) => write!(out, r#"],"type":"all"}}"#),
            Visit::Exit(NativeScript::ScriptAny(_)) => write!(out, r#"],"type":"any"}}"#),
            Visit::Exit(NativeScript::ScriptNOfK(..)) => write!(out, r#"],"type":"atLeast"}}"#),
            Visit::Exit(_) => Ok(()),
        })
        .expect("writing to a String cannot fail");
        out
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use crate::{ToCanonicalJson, alonzo::Block};

    type BlockWrapper<'a> = (u16, Block<'a>);

    #[test]
    fn test_datums_serialize_as_expected() {
        let test_blocks = [(
            include_str!("../../../test_data/alonzo9.block"),
            include_str!("../../../test_data/alonzo9.datums"),
        )];

        for (idx, (block_str, jsonl_str)) in test_blocks.iter().enumerate() {
            println!("decoding json block {}", idx + 1);

            let bytes = hex::decode(block_str).unwrap_or_else(|_| panic!("bad block file {idx}"));

            let (_, block): BlockWrapper = minicbor::decode(&bytes[..])
                .unwrap_or_else(|_| panic!("error decoding cbor for file {idx}"));

            let mut datums = jsonl_str.lines();

            for ws in block.transaction_witness_sets.iter() {
                if let Some(pds) = &ws.plutus_data {
                    for pd in pds.iter() {
                        let expected: serde_json::Value =
                            serde_json::from_str(datums.next().unwrap()).unwrap();
                        let current = pd.to_json();
                        assert_eq!(current, expected);
                    }
                }
            }
        }
    }

    #[test]
    fn test_native_scripts_serialize_as_expected() {
        let test_blocks = [(
            include_str!("../../../test_data/alonzo9.block"),
            include_str!("../../../test_data/alonzo9.native"),
        )];

        for (idx, (block_str, jsonl_str)) in test_blocks.iter().enumerate() {
            println!("decoding json block {}", idx + 1);

            let bytes = hex::decode(block_str).unwrap_or_else(|_| panic!("bad block file {idx}"));

            let (_, block): BlockWrapper = minicbor::decode(&bytes[..])
                .unwrap_or_else(|_| panic!("error decoding cbor for file {idx}"));

            let mut scripts = jsonl_str.lines();

            for ws in block.transaction_witness_sets.iter() {
                if let Some(nss) = &ws.native_script {
                    for ns in nss.iter() {
                        let expected: serde_json::Value =
                            serde_json::from_str(scripts.next().unwrap()).unwrap();
                        let current = ns.to_json();
                        assert_eq!(current, expected);

                        let text: serde_json::Value =
                            serde_json::from_str(&ns.to_json_string()).unwrap();
                        assert_eq!(text, expected);
                    }
                }
            }
        }
    }

    #[test]
    fn native_script_json_preserves_mixed_shape_trees() {
        use crate::alonzo::NativeScript;
        use serde_json::json;

        // Width > 1 at more than one level: the iterative conversion pairs
        // mapped children with source children positionally.
        let script = NativeScript::ScriptNOfK(
            2,
            vec![
                NativeScript::ScriptPubkey([1; 28].into()),
                NativeScript::ScriptAll(vec![
                    NativeScript::ScriptPubkey([2; 28].into()),
                    NativeScript::ScriptAny(vec![
                        NativeScript::InvalidBefore(100),
                        NativeScript::InvalidHereafter(200),
                    ]),
                ]),
                NativeScript::ScriptPubkey([3; 28].into()),
            ],
        );

        let expected = json!({
            "type": "atLeast",
            "required": 2,
            "scripts": [
                { "type": "sig", "keyHash": hex::encode([1u8; 28]) },
                { "type": "all", "scripts": [
                    { "type": "sig", "keyHash": hex::encode([2u8; 28]) },
                    { "type": "any", "scripts": [
                        { "type": "after", "slot": 100 },
                        { "type": "before", "slot": 200 },
                    ]},
                ]},
                { "type": "sig", "keyHash": hex::encode([3u8; 28]) },
            ],
        });

        assert_eq!(script.to_json(), expected);

        let text: serde_json::Value = serde_json::from_str(&script.to_json_string()).unwrap();
        assert_eq!(text, expected);
    }

    #[test]
    fn native_script_json_string_handles_deeply_nested_scripts_on_a_small_stack() {
        use crate::alonzo::NativeScript;

        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let mut script = NativeScript::ScriptPubkey([0; 28].into());
                for _ in 0..20_000 {
                    script = NativeScript::ScriptAll(vec![script]);
                }

                let expected = format!(
                    "{}{{\"keyHash\":\"{}\",\"type\":\"sig\"}}{}",
                    r#"{"scripts":["#.repeat(20_000),
                    hex::encode([0u8; 28]),
                    r#"],"type":"all"}"#.repeat(20_000),
                );
                assert_eq!(script.to_json_string(), expected);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn native_script_json_handles_deeply_nested_scripts_on_a_small_stack() {
        use crate::alonzo::NativeScript;

        // Depth and stack size are load-bearing: the recursive conversion
        // (one call frame per level) aborts here.
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let mut script = NativeScript::ScriptPubkey([0; 28].into());
                for _ in 0..20_000 {
                    script = NativeScript::ScriptAll(vec![script]);
                }

                // serde_json::Value has no stack-safe Drop, so dropping a
                // chain this deep (including on a failed assertion) would
                // abort. Leak it: this test is only about the conversion.
                let json = std::mem::ManuallyDrop::new(script.to_json());

                let mut depth = 0;
                let mut cursor: &serde_json::Value = &json;
                while let Some(scripts) = cursor.get("scripts") {
                    cursor = scripts.get(0).expect("all must carry a child");
                    depth += 1;
                }
                assert_eq!(depth, 20_000);
                assert_eq!(cursor["type"], "sig");
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
