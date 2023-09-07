use std::ops::Deref;

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
        match self {
            super::NativeScript::ScriptPubkey(x) => {
                json!({ "keyHash": x.to_string(), "type": "sig"})
            }
            super::NativeScript::ScriptAll(x) => {
                let scripts: Vec<_> = x.iter().map(|i| i.to_json()).collect();
                json!({ "type": "all", "scripts": scripts})
            }
            super::NativeScript::ScriptAny(x) => {
                let scripts: Vec<_> = x.iter().map(|i| i.to_json()).collect();
                json!({ "type": "any", "scripts": scripts})
            }
            super::NativeScript::ScriptNOfK(n, k) => {
                let scripts: Vec<_> = k.iter().map(|i| i.to_json()).collect();
                json!({ "type": "atLeast", "required": n, "scripts" : scripts })
            }
            super::NativeScript::InvalidBefore(slot) => json!({ "type": "after", "slot": slot }),
            super::NativeScript::InvalidHereafter(slot) => json!({"type": "before", "slot": slot }),
        }
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;

    use crate::{alonzo::Block, ToCanonicalJson};

    type BlockWrapper = (u16, Block);

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
                    }
                }
            }
        }
    }
}
