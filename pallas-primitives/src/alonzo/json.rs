use serde_json::json;

use crate::ToCanonicalJson;

// infered from https://github.com/input-output-hk/cardano-node/blob/c1efb2f97134c0607c982246a36e3da7266ac194/cardano-api/src/Cardano/Api/ScriptData.hs#L254
impl ToCanonicalJson for super::PlutusData {
    fn to_json(&self) -> serde_json::Value {
        match self {
            super::PlutusData::Constr(x) => {
                let constructor = x.prefix.map(|x| x as u64).unwrap_or(x.tag);
                let fields: Vec<_> = x.values.iter().map(|i| i.to_json()).collect();
                json!({ "constructor": constructor, "fields": fields })
            }
            super::PlutusData::Map(x) => {
                let map: Vec<_> = x
                    .iter()
                    .map(|(k, v)| json!({ "k": k.to_json(), "v": v.to_json() }))
                    .collect();
                json!({ "map": map })
            }
            super::PlutusData::BigInt(int) => match int {
                super::BigInt::Int(n) => json!({ "int": i128::from(*n) }),
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
            super::PlutusData::ArrayIndef(x) => {
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
            super::NativeScript::InvalidBefore(slot) => json!({ "type": "before", "slot": slot }),
            super::NativeScript::InvalidHereafter(slot) => json!({"type": "after", "slot": slot }),
        }
    }
}
