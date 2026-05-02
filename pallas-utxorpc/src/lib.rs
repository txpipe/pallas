use std::collections::HashMap;

use pallas_crypto::hash::Hash;
use pallas_primitives::alonzo;
use pallas_traverse as trv;

#[cfg(any(feature = "v1alpha", feature = "v1beta"))]
#[macro_use]
mod shared;

#[cfg(feature = "v1alpha")]
pub mod v1alpha;

#[cfg(feature = "v1beta")]
pub mod v1beta;

#[cfg(feature = "v1alpha")]
pub use v1alpha::{spec, Mapper};

pub type TxHash = Hash<32>;
pub type TxoIndex = u32;
pub type TxoRef = (TxHash, TxoIndex);
pub type Cbor = Vec<u8>;
pub type EraCbor = (trv::Era, Cbor);
pub type UtxoMap = HashMap<TxoRef, EraCbor>;
pub type DatumMap = HashMap<Hash<32>, alonzo::PlutusData>;

pub trait LedgerContext: Clone {
    fn get_utxos(&self, refs: &[TxoRef]) -> Option<UtxoMap>;
    fn get_slot_timestamp(&self, slot: u64) -> Option<u64>;
}

/// Wire-compatibility checks for messages that are expected to be byte-identical
/// between v1alpha::cardano and v1beta::cardano. If any of these fail, the
/// shared mapping macro in `shared.rs` can no longer assume the type emits the
/// same bytes for both versions and must move the affected method per-version.
///
/// Block / Tx / TxOutput / Asset / Multiasset / Datum / WitnessSet / GovernanceAction
/// are intentionally NOT checked here — they are known to diverge between
/// versions, which is why those mappers live in per-version files.
#[cfg(all(test, feature = "v1alpha", feature = "v1beta"))]
mod cross_version_compat {
    use prost::Message;

    use utxorpc_spec::utxorpc::v1alpha::cardano as v1a;
    use utxorpc_spec::utxorpc::v1beta::cardano as v1b;

    fn assert_wire_compat<A: Message + Default, B: Message + Default>(label: &str, a: A) {
        let bytes = a.encode_to_vec();
        let b = B::decode(bytes.as_slice())
            .unwrap_or_else(|e| panic!("{label}: v1alpha → v1beta decode failed: {e}"));
        let round = b.encode_to_vec();
        assert_eq!(
            bytes, round,
            "{label}: v1alpha bytes differ from v1alpha → v1beta → bytes round-trip; \
             this means the wire format diverged and the shared macro must move this \
             method per-version"
        );
    }

    #[test]
    fn pparams_wire_compatible() {
        assert_wire_compat::<v1a::PParams, v1b::PParams>(
            "PParams",
            v1a::PParams::default(),
        );
    }

    #[test]
    fn certificate_wire_compatible() {
        assert_wire_compat::<v1a::Certificate, v1b::Certificate>(
            "Certificate",
            v1a::Certificate::default(),
        );
    }

    #[test]
    fn pool_registration_cert_wire_compatible() {
        assert_wire_compat::<v1a::PoolRegistrationCert, v1b::PoolRegistrationCert>(
            "PoolRegistrationCert",
            v1a::PoolRegistrationCert::default(),
        );
    }

    #[test]
    fn metadata_wire_compatible() {
        assert_wire_compat::<v1a::Metadata, v1b::Metadata>(
            "Metadata",
            v1a::Metadata::default(),
        );
    }

    #[test]
    fn plutus_data_wire_compatible() {
        assert_wire_compat::<v1a::PlutusData, v1b::PlutusData>(
            "PlutusData",
            v1a::PlutusData::default(),
        );
    }
}
