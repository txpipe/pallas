//! Map Pallas ledger types onto the [UTxORPC] Cardano protobuf schema.
//!
//! Both spec versions are always compiled in side by side, so a caller can
//! choose `v1alpha` or `v1beta` per call site without juggling features.
//!
//! [UTxORPC]: https://utxorpc.org
//!
//! # Usage
//!
//! ```
//! use pallas_utxorpc::Mapper;          // default features: alias for v1alpha::Mapper
//! use pallas_utxorpc::v1alpha::Mapper as V1Alpha;
//! use pallas_utxorpc::v1beta::Mapper as V1Beta;
//! ```
//!
//! For back-compat with pre-v1beta releases, the default
//! `u5c-v1alpha-compat` feature re-exports `v1alpha` at the crate root, so
//! `pallas_utxorpc::Mapper` and `pallas_utxorpc::spec` keep resolving to
//! v1alpha. Disable the compat shim to force callers onto explicit version
//! paths:
//!
//! ```toml
//! pallas-utxorpc = { version = "...", default-features = false }
//! ```
//!
//! # Overview
//!
//! - [`v1alpha`] — `Mapper` returning
//!   `utxorpc_spec::utxorpc::v1alpha::cardano::*`.
//! - [`v1beta`] — `Mapper` returning
//!   `utxorpc_spec::utxorpc::v1beta::cardano::*`, including the v1beta-only
//!   types (`BootstrapWitness`, `VoterVotes`, `VotingProcedure`, `Vote`).
//! - Crate-root infrastructure ([`LedgerContext`], [`TxHash`], [`TxoIndex`],
//!   [`TxoRef`], [`Cbor`], [`EraCbor`], [`UtxoMap`], [`DatumMap`]) is
//!   shared across versions and unaffected by the feature flag.
//!
//! # Feature flags
//!
//! - `u5c-v1alpha-compat` *(default)* — re-export [`v1alpha::Mapper`] and
//!   [`v1alpha::spec`] at the crate root for back-compat with pre-v1beta
//!   callers.
//!
//! # Testing
//!
//! Each version has a snapshot test that decodes a fixed Babbage block and
//! compares the mapper output against a JSON file under `test_data/`
//! (`u5c_v1alpha.json`, `u5c_v1beta.json`). To overwrite both snapshots with
//! the current mapper output:
//!
//! ```sh
//! REGENERATE_SNAPSHOTS=1 cargo test -p pallas-utxorpc
//! ```
//!
//! When the variable is unset (the normal case), the tests assert against
//! the checked-in JSON files.
//!
//! # Usage as part of `pallas`
//!
//! When depending on the umbrella [`pallas`] crate, this crate is re-exported
//! as `pallas::interop::utxorpc`.
//!
//! [`pallas`]: https://crates.io/crates/pallas

use std::collections::HashMap;

use pallas_crypto::hash::Hash;
use pallas_primitives::alonzo;
use pallas_traverse as trv;

#[macro_use]
mod shared;

/// Mappers and types for the `v1alpha` UTxO RPC schema.
pub mod v1alpha;
/// Mappers and types for the `v1beta` UTxO RPC schema.
pub mod v1beta;

#[cfg(feature = "u5c-v1alpha-compat")]
pub use v1alpha::{spec, Mapper};

/// 32-byte transaction hash.
pub type TxHash = Hash<32>;
/// Index of an output within a transaction.
pub type TxoIndex = u32;
/// Reference to a single transaction output: `(tx_hash, output_index)`.
pub type TxoRef = (TxHash, TxoIndex);
/// Raw CBOR bytes for an on-chain artifact.
pub type Cbor = Vec<u8>;
/// CBOR bytes tagged with the era they were produced in.
pub type EraCbor = (trv::Era, Cbor);
/// Resolved UTxO set keyed by output reference.
pub type UtxoMap = HashMap<TxoRef, EraCbor>;
/// Plutus datums keyed by their 32-byte hash.
pub type DatumMap = HashMap<Hash<32>, alonzo::PlutusData>;

/// Side-channel a UTxO RPC mapper uses to resolve information that is not
/// inlined in the transaction or block being mapped (referenced UTxOs, slot
/// timestamps, etc.).
pub trait LedgerContext: Clone {
    /// Resolve a set of output references to their on-chain CBOR.
    fn get_utxos(&self, refs: &[TxoRef]) -> Option<UtxoMap>;
    /// Resolve a slot number to its wall-clock timestamp (UNIX seconds).
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
#[cfg(test)]
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
        assert_wire_compat::<v1a::PParams, v1b::PParams>("PParams", v1a::PParams::default());
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
        assert_wire_compat::<v1a::Metadata, v1b::Metadata>("Metadata", v1a::Metadata::default());
    }

    #[test]
    fn plutus_data_wire_compatible() {
        assert_wire_compat::<v1a::PlutusData, v1b::PlutusData>(
            "PlutusData",
            v1a::PlutusData::default(),
        );
    }
}
