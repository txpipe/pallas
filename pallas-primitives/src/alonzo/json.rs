use std::ops::Deref;

use pallas_codec::tree::{IndexedNode, Visit, fold_tree, walk_tree};
use serde_json::{Value, json};

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

/// An object whose values are moved in. `json!` would re-serialize them,
/// which recurses through the whole subtree.
fn object(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut map = serde_json::Map::new();
    for (key, value) in entries {
        map.insert(key.to_string(), value);
    }
    Value::Object(map)
}

/// The JSON of a leaf datum, shared by both renderings.
fn plutus_leaf_json(x: &super::PlutusData) -> Option<serde_json::Value> {
    let value = match x {
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
        _ => return None,
    };
    Some(value)
}

// infered from https://github.com/input-output-hk/cardano-node/blob/c1efb2f97134c0607c982246a36e3da7266ac194/cardano-api/src/Cardano/Api/ScriptData.hs#L254
impl ToCanonicalJson for super::PlutusData {
    /// Building the tree is stack-safe, but the returned `serde_json::Value`
    /// is not: serializing, cloning, comparing and dropping it recurse per
    /// nesting level, and a chain-deep datum overflows a 2 MiB stack doing
    /// so. Use [`to_json_string`](ToCanonicalJson::to_json_string) when the
    /// depth is not under your control.
    fn to_json(&self) -> serde_json::Value {
        use super::PlutusData;

        fold_tree(self, |node, children: Vec<Value>| match node {
            PlutusData::Constr(x) => object([
                ("constructor", json!(x.constructor_value())),
                ("fields", Value::Array(children)),
            ]),
            PlutusData::Map(_) => {
                let mut children = children.into_iter();
                let mut map = Vec::with_capacity(children.len() / 2);
                while let (Some(k), Some(v)) = (children.next(), children.next()) {
                    map.push(object([("k", k), ("v", v)]));
                }
                object([("map", Value::Array(map))])
            }
            PlutusData::Array(_) => object([("list", Value::Array(children))]),
            leaf => plutus_leaf_json(leaf).expect("leaf variant"),
        })
    }

    /// Writes the JSON text directly, never building a `serde_json::Value`,
    /// so no step of rendering a chain-deep datum can overflow the stack.
    fn to_json_string(&self) -> String {
        use std::fmt::Write;

        use super::PlutusData;

        let mut out = String::new();
        // Children rendered so far of each open container, innermost last;
        // a map alternates keys and values and needs to know which is next.
        let mut rendered: Vec<usize> = Vec::new();
        walk_tree::<_, std::fmt::Error>(self, |visit| match visit {
            Visit::Enter(PlutusData::Constr(x)) => {
                rendered.push(0);
                match x.constructor_value() {
                    Some(n) => write!(out, r#"{{"constructor":{n},"fields":["#),
                    None => write!(out, r#"{{"constructor":null,"fields":["#),
                }
            }
            Visit::Enter(node @ PlutusData::Map(_)) => {
                rendered.push(0);
                write!(out, r#"{{"map":["#)?;
                if node.child_count() > 0 {
                    write!(out, r#"{{"k":"#)?;
                }
                Ok(())
            }
            Visit::Enter(PlutusData::Array(_)) => {
                rendered.push(0);
                write!(out, r#"{{"list":["#)
            }
            Visit::Enter(leaf) => {
                write!(out, "{}", plutus_leaf_json(leaf).expect("leaf variant"))
            }
            Visit::Between(PlutusData::Map(_)) => {
                let count = rendered.last_mut().expect("inside a map");
                *count += 1;
                if count.is_multiple_of(2) {
                    write!(out, r#"}},{{"k":"#)
                } else {
                    write!(out, r#","v":"#)
                }
            }
            Visit::Between(_) => write!(out, ","),
            Visit::Exit(node @ PlutusData::Map(_)) => {
                rendered.pop();
                if node.child_count() > 0 {
                    write!(out, "}}]}}")
                } else {
                    write!(out, "]}}")
                }
            }
            Visit::Exit(PlutusData::Constr(_) | PlutusData::Array(_)) => {
                rendered.pop();
                write!(out, "]}}")
            }
            Visit::Exit(_) => Ok(()),
        })
        .expect("writing to a String cannot fail");
        out
    }
}

impl ToCanonicalJson for super::NativeScript {
    /// Building the tree is stack-safe, but the returned `serde_json::Value`
    /// is not: serializing, cloning, comparing and dropping it recurse per
    /// nesting level, and a chain-deep script overflows a 2 MiB stack doing
    /// so. Use [`to_json_string`](ToCanonicalJson::to_json_string) when the
    /// depth is not under your control.
    fn to_json(&self) -> serde_json::Value {
        use super::NativeScript;

        fold_tree(self, |node, children: Vec<Value>| match node {
            NativeScript::ScriptPubkey(x) => json!({ "keyHash": x.to_string(), "type": "sig"}),
            NativeScript::ScriptAll(_) => {
                object([("scripts", Value::Array(children)), ("type", json!("all"))])
            }
            NativeScript::ScriptAny(_) => {
                object([("scripts", Value::Array(children)), ("type", json!("any"))])
            }
            NativeScript::ScriptNOfK(n, _) => object([
                ("required", json!(n)),
                ("scripts", Value::Array(children)),
                ("type", json!("atLeast")),
            ]),
            NativeScript::InvalidBefore(slot) => json!({ "type": "after", "slot": slot }),
            NativeScript::InvalidHereafter(slot) => json!({"type": "before", "slot": slot }),
        })
    }

    /// Writes the JSON text directly, never building a `serde_json::Value`,
    /// so no step of rendering a chain-deep script can overflow the stack.
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

                        let text = pd.to_json_string();
                        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
                        assert_eq!(parsed, expected);
                        assert_eq!(text, current.to_string());
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

    fn deep_datum_shapes(depth: usize) -> Vec<(crate::PlutusData, String)> {
        use crate::{BigInt, Constr, Int, KeyValuePairs, MaybeIndefArray, PlutusData};

        let int = |n: i64| PlutusData::BigInt(BigInt::Int(Int::from(n)));
        let mut constr = int(0);
        let mut map = int(0);
        let mut list = int(0);
        for _ in 0..depth {
            constr = PlutusData::Constr(Constr {
                tag: 121,
                any_constructor: None,
                fields: MaybeIndefArray::Indef(vec![constr]),
            });
            map = PlutusData::Map(KeyValuePairs::Def(vec![(int(1), map)]));
            list = PlutusData::Array(MaybeIndefArray::Def(vec![list]));
        }
        let leaf = r#"{"int":0}"#;
        vec![
            (
                constr,
                format!(
                    "{}{leaf}{}",
                    r#"{"constructor":0,"fields":["#.repeat(depth),
                    "]}".repeat(depth)
                ),
            ),
            (
                map,
                format!(
                    "{}{leaf}{}",
                    r#"{"map":[{"k":{"int":1},"v":"#.repeat(depth),
                    "}]}".repeat(depth)
                ),
            ),
            (
                list,
                format!(
                    "{}{leaf}{}",
                    r#"{"list":["#.repeat(depth),
                    "]}".repeat(depth)
                ),
            ),
        ]
    }

    #[test]
    fn plutus_data_json_preserves_mixed_shapes() {
        use crate::PlutusData;

        // Constr(102, 5) [Map [(1, [])], []_, Map [], b"", 2^64, Constr 0 []]
        let bytes = hex::decode("d866820586a101809fffa040c249010000000000000000d87980").unwrap();
        let data: PlutusData = minicbor::decode(&bytes).unwrap();
        let expected = r#"{"constructor":5,"fields":[{"map":[{"k":{"int":1},"v":{"list":[]}}]},{"list":[]},{"map":[]},{"bytes":""},{"biguint":"010000000000000000"},{"constructor":0,"fields":[]}]}"#;
        assert_eq!(data.to_json_string(), expected);
        assert_eq!(data.to_json().to_string(), expected);
    }

    #[test]
    fn plutus_data_json_string_handles_deeply_nested_data_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                for (data, expected) in deep_datum_shapes(20_000) {
                    // PlutusData's Drop still recurses; leak the value.
                    let data = std::mem::ManuallyDrop::new(data);
                    assert_eq!(data.to_json_string(), expected);
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn plutus_data_json_handles_deeply_nested_data_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                for (data, _) in deep_datum_shapes(20_000) {
                    let data = std::mem::ManuallyDrop::new(data);
                    // Neither the datum nor serde_json::Value has a
                    // stack-safe Drop; leak both, the conversion is the test.
                    let json = std::mem::ManuallyDrop::new(data.to_json());

                    let mut depth = 0;
                    let mut cursor: &serde_json::Value = &json;
                    loop {
                        let next = if let Some(fields) = cursor.get("fields") {
                            fields.get(0)
                        } else if let Some(map) = cursor.get("map") {
                            map.get(0).and_then(|pair| pair.get("v"))
                        } else if let Some(list) = cursor.get("list") {
                            list.get(0)
                        } else {
                            break;
                        };
                        cursor = next.expect("container carries a child");
                        depth += 1;
                    }
                    assert_eq!(depth, 20_000);
                    assert_eq!(cursor["int"], 0);
                }
            })
            .unwrap()
            .join()
            .unwrap();
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
