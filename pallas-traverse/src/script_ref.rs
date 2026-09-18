use std::{borrow::Cow, ops::Deref};

use pallas_codec::minicbor;
use pallas_crypto::hash::Hash;
use pallas_primitives::conway;

#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

use crate::{ComputeHash as _, Era, MultiEraNativeScript, MultiEraScriptRef, OriginalHash as _};

impl<'b> MultiEraScriptRef<'b> {
    /// Read a Babbage or Conway reference script.
    pub fn from_conway(script: &'b conway::ScriptRef<'b>) -> Self {
        Self::Conway(Cow::Borrowed(script))
    }

    #[cfg(feature = "unstable")]
    pub fn from_dijkstra(script: &'b dijkstra::ScriptRef<'b>) -> Self {
        Self::Dijkstra(Cow::Borrowed(script))
    }

    pub fn as_conway(&self) -> Option<&conway::ScriptRef<'b>> {
        match self {
            Self::Conway(x) => Some(x),
            #[cfg(feature = "unstable")]
            Self::Dijkstra(_) => None,
        }
    }

    #[cfg(feature = "unstable")]
    pub fn as_dijkstra(&self) -> Option<&dijkstra::ScriptRef<'b>> {
        match self {
            Self::Dijkstra(x) => Some(x),
            Self::Conway(_) => None,
        }
    }

    /// Returns the era whose `script` rule carries this value. The Conway
    /// variant serves Babbage too.
    pub fn era(&self) -> Era {
        match self {
            Self::Conway(_) => Era::Conway,
            #[cfg(feature = "unstable")]
            Self::Dijkstra(_) => Era::Dijkstra,
        }
    }

    pub fn language(&self) -> ScriptLanguage {
        match self {
            Self::Conway(x) => match x.deref() {
                conway::ScriptRef::NativeScript(_) => ScriptLanguage::Native,
                conway::ScriptRef::PlutusV1Script(_) => ScriptLanguage::PlutusV1,
                conway::ScriptRef::PlutusV2Script(_) => ScriptLanguage::PlutusV2,
                conway::ScriptRef::PlutusV3Script(_) => ScriptLanguage::PlutusV3,
            },
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => match x.deref() {
                dijkstra::ScriptRef::NativeScript(_) => ScriptLanguage::Native,
                dijkstra::ScriptRef::PlutusV1Script(_) => ScriptLanguage::PlutusV1,
                dijkstra::ScriptRef::PlutusV2Script(_) => ScriptLanguage::PlutusV2,
                dijkstra::ScriptRef::PlutusV3Script(_) => ScriptLanguage::PlutusV3,
                dijkstra::ScriptRef::PlutusV4Script(_) => ScriptLanguage::PlutusV4,
            },
        }
    }

    pub fn native_script(&self) -> Option<MultiEraNativeScript<'_>> {
        match self {
            Self::Conway(x) => match x.deref() {
                conway::ScriptRef::NativeScript(s) => {
                    Some(MultiEraNativeScript::from_alonzo_compatible(s))
                }
                _ => None,
            },
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => match x.deref() {
                dijkstra::ScriptRef::NativeScript(s) => {
                    Some(MultiEraNativeScript::from_dijkstra(s))
                }
                _ => None,
            },
        }
    }

    pub fn plutus_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Conway(x) => match x.deref() {
                conway::ScriptRef::NativeScript(_) => None,
                conway::ScriptRef::PlutusV1Script(s) => Some(s.0.as_ref()),
                conway::ScriptRef::PlutusV2Script(s) => Some(s.0.as_ref()),
                conway::ScriptRef::PlutusV3Script(s) => Some(s.0.as_ref()),
            },
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => match x.deref() {
                dijkstra::ScriptRef::NativeScript(_) => None,
                dijkstra::ScriptRef::PlutusV1Script(s) => Some(s.0.as_ref()),
                dijkstra::ScriptRef::PlutusV2Script(s) => Some(s.0.as_ref()),
                dijkstra::ScriptRef::PlutusV3Script(s) => Some(s.0.as_ref()),
                dijkstra::ScriptRef::PlutusV4Script(s) => Some(s.0.as_ref()),
            },
        }
    }

    /// Returns the script hash the ledger keys this script by, blake2b-224
    /// behind its language tag. A native script hashes its original CBOR, so a
    /// non canonical encoding still gives the committed hash.
    pub fn hash(&self) -> Hash<28> {
        match self {
            Self::Conway(x) => match x.deref() {
                conway::ScriptRef::NativeScript(s) => s.original_hash(),
                conway::ScriptRef::PlutusV1Script(s) => s.compute_hash(),
                conway::ScriptRef::PlutusV2Script(s) => s.compute_hash(),
                conway::ScriptRef::PlutusV3Script(s) => s.compute_hash(),
            },
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => match x.deref() {
                dijkstra::ScriptRef::NativeScript(s) => s.original_hash(),
                dijkstra::ScriptRef::PlutusV1Script(s) => s.compute_hash(),
                dijkstra::ScriptRef::PlutusV2Script(s) => s.compute_hash(),
                dijkstra::ScriptRef::PlutusV3Script(s) => s.compute_hash(),
                dijkstra::ScriptRef::PlutusV4Script(s) => s.compute_hash(),
            },
        }
    }

    /// Returns the whole `script` CBOR, in the era's own encoding.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Conway(x) => minicbor::to_vec(x).expect("to_vec is infallible"),
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => minicbor::to_vec(x).expect("to_vec is infallible"),
        }
    }

    /// Read a `script` in the era's own encoding. The era is a parameter
    /// because variants 0 through 3 are identical and only Dijkstra admits 4.
    pub fn decode(era: Era, cbor: &'b [u8]) -> Result<Self, minicbor::decode::Error> {
        match era {
            #[cfg(feature = "unstable")]
            Era::Dijkstra => Ok(Self::Dijkstra(Cow::Owned(minicbor::decode(cbor)?))),
            Era::Babbage | Era::Conway => Ok(Self::Conway(Cow::Owned(minicbor::decode(cbor)?))),
            Era::Byron | Era::Shelley | Era::Allegra | Era::Mary | Era::Alonzo => Err(
                minicbor::decode::Error::message(format!("{era} has no reference script rule")),
            ),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScriptLanguage {
    Native,
    PlutusV1,
    PlutusV2,
    PlutusV3,
    /// Plutus V4, new in Dijkstra.
    PlutusV4,
}

impl ScriptLanguage {
    /// Returns the tag byte the ledger prefixes to a script before hashing it.
    pub fn tag(&self) -> u8 {
        match self {
            Self::Native => 0,
            Self::PlutusV1 => 1,
            Self::PlutusV2 => 2,
            Self::PlutusV3 => 3,
            Self::PlutusV4 => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MultiEraTx, testing};
    use pallas_crypto::hash::Hasher;

    const ADDRESS: [u8; 29] = [0x60; 29];
    const V2_BYTES: [u8; 3] = [0x4d, 0x01, 0x00];

    fn conway_output_with_script(script: &[u8]) -> Vec<u8> {
        let output = testing::post_alonzo_output_with_script_ref(&ADDRESS, 2_000_000, script);
        testing::conway_tx(
            &testing::body_with_output(&output),
            &testing::empty_witness_set(),
            None,
            true,
        )
    }

    #[test]
    fn a_conway_reference_script_is_read_in_the_shared_type() {
        let script = testing::script_ref_plutus_v2(&V2_BYTES);
        let cbor = conway_output_with_script(&script);
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");

        let outputs = tx.outputs();
        let found = outputs[0]
            .script_ref()
            .expect("a written reference script must be readable");

        assert_eq!(found.era(), Era::Conway);
        assert_eq!(found.language(), ScriptLanguage::PlutusV2);
        assert_eq!(found.language().tag(), 2);
        assert_eq!(found.plutus_bytes(), Some(V2_BYTES.as_slice()));
        assert!(found.native_script().is_none());
        assert_eq!(
            hex::encode(found.encode()),
            hex::encode(&script),
            "the reference script re-encodes to the bytes it was read from"
        );
        assert_eq!(
            found.hash(),
            Hasher::<224>::hash_tagged(&V2_BYTES, 2),
            "a V2 script hashes behind tag 2"
        );
    }

    #[test]
    fn a_conway_native_reference_script_is_read_in_the_shared_type() {
        let native = testing::native_script_pubkey(0x5c);
        let mut e = pallas_codec::minicbor::Encoder::new(Vec::new());
        e.array(2).unwrap();
        e.u8(0).unwrap();
        e.writer_mut().extend_from_slice(&native);
        let script = e.into_writer();

        let cbor = conway_output_with_script(&script);
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");
        let outputs = tx.outputs();
        let found = outputs[0].script_ref().expect("readable");

        assert_eq!(found.language(), ScriptLanguage::Native);
        assert!(found.plutus_bytes().is_none());

        let inner = found.native_script().expect("a native script");
        assert_eq!(inner.era(), Era::Alonzo);
        assert_eq!(hex::encode(inner.encode()), hex::encode(&native));
    }

    #[test]
    fn a_conway_output_with_no_reference_script_reports_none() {
        let output = {
            let mut e = pallas_codec::minicbor::Encoder::new(Vec::new());
            e.array(2).unwrap();
            e.bytes(&ADDRESS).unwrap();
            e.u64(2_000_000).unwrap();
            e.into_writer()
        };
        let cbor = testing::conway_tx(
            &testing::body_with_output(&output),
            &testing::empty_witness_set(),
            None,
            true,
        );
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");

        assert!(tx.outputs()[0].script_ref().is_none());
    }
}

#[cfg(all(test, feature = "unstable"))]
mod dijkstra_tests {
    use super::*;
    use crate::{MultiEraTx, testing};
    use pallas_crypto::hash::Hasher;

    const ADDRESS: [u8; 29] = [0x60; 29];
    const V4_BYTES: [u8; 3] = [0xd8, 0x79, 0x80];

    fn dijkstra_output_with_script(script: &[u8]) -> Vec<u8> {
        let output = testing::post_alonzo_output_with_script_ref(&ADDRESS, 2_000_000, script);
        testing::dijkstra_block_tx(
            &testing::body_with_output(&output),
            &testing::empty_witness_set(),
            None,
            true,
        )
    }

    #[test]
    fn a_plutus_v4_reference_script_is_reported_as_v4() {
        let cbor = dijkstra_output_with_script(&testing::script_ref_plutus_v4(&V4_BYTES));
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("must decode");

        let output = &tx.outputs()[0];
        let script = output
            .script_ref()
            .expect("a written reference script must be readable");

        assert_eq!(script.era(), Era::Dijkstra);
        assert_eq!(script.language(), ScriptLanguage::PlutusV4);
        assert_eq!(script.language().tag(), 4);
        assert_eq!(script.plutus_bytes(), Some(V4_BYTES.as_slice()));
        assert!(script.native_script().is_none());
        assert!(script.as_conway().is_none());
        assert!(script.as_dijkstra().is_some());

        assert_eq!(
            script.hash(),
            Hasher::<224>::hash_tagged(&V4_BYTES, 4),
            "a V4 script hashes behind tag 4"
        );

        assert_eq!(
            hex::encode(script.encode()),
            hex::encode(testing::script_ref_plutus_v4(&V4_BYTES))
        );
        let encoded = script.encode();
        let round = MultiEraScriptRef::decode(Era::Dijkstra, &encoded)
            .expect("a script re-encoded here must decode again");
        assert_eq!(round.language(), ScriptLanguage::PlutusV4);
    }

    #[test]
    fn a_native_reference_script_is_reported_as_native() {
        let native = testing::native_script_require_guard(0x11);
        let mut e = pallas_codec::minicbor::Encoder::new(Vec::new());
        e.array(2).unwrap();
        e.u8(0).unwrap();
        e.writer_mut().extend_from_slice(&native);
        let script_ref = e.into_writer();

        let cbor = dijkstra_output_with_script(&script_ref);
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("must decode");

        let output = &tx.outputs()[0];
        let script = output.script_ref().expect("readable");

        assert_eq!(script.language(), ScriptLanguage::Native);
        assert_eq!(script.language().tag(), 0);
        assert!(script.plutus_bytes().is_none());

        let inner = script.native_script().expect("a native script");
        assert_eq!(inner.era(), Era::Dijkstra);
        assert_eq!(hex::encode(inner.encode()), hex::encode(&native));

        assert_eq!(
            script.hash(),
            Hasher::<224>::hash_tagged(&native, 0),
            "a native script hashes its own CBOR behind tag 0"
        );
    }

    #[test]
    fn an_output_with_no_reference_script_reports_none() {
        let output = {
            let mut e = pallas_codec::minicbor::Encoder::new(Vec::new());
            e.array(2).unwrap();
            e.bytes(&ADDRESS).unwrap();
            e.u64(2_000_000).unwrap();
            e.into_writer()
        };
        let cbor = testing::dijkstra_block_tx(
            &testing::body_with_output(&output),
            &testing::empty_witness_set(),
            None,
            true,
        );
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("must decode");

        assert!(tx.outputs()[0].script_ref().is_none());
        assert!(tx.outputs()[0].datum().is_none());
        assert_eq!(tx.outputs()[0].value().coin(), 2_000_000);
    }

    #[test]
    fn an_era_with_no_reference_script_rule_is_refused() {
        let bytes = testing::script_ref_plutus_v4(&V4_BYTES);

        for era in [
            Era::Byron,
            Era::Shelley,
            Era::Allegra,
            Era::Mary,
            Era::Alonzo,
        ] {
            assert!(
                MultiEraScriptRef::decode(era, &bytes).is_err(),
                "{era} has no reference script rule"
            );
        }

        assert!(MultiEraScriptRef::decode(Era::Conway, &bytes).is_err());
    }
}
