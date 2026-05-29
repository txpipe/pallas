//! Integration tests for the Conway transaction builder.
//!
//! These tests pin down the *current* behaviour of `build_conway_raw` and
//! `BuiltTransaction::sign` so that future changes are deliberate and visible.
//!
//! Test oracles used here (no live node required):
//!
//! 1. **Decode round-trip** — every built transaction is decoded back with
//!    `pallas_primitives::conway::Tx` and its fields are asserted against the
//!    inputs we gave the builder.
//! 2. **Golden snapshots** — the CBOR hex of representative transactions is
//!    pinned as a constant; any change to the encoder shows up as a diff.
//! 3. **Determinism** — the same logical transaction, built more than once
//!    (and with collections populated in different orders), must encode to
//!    identical bytes.
//! 4. **Script-data self-consistency** — the `script_data_hash` embedded by
//!    the builder is recomputed from the decoded witness set via
//!    `pallas_primitives::conway::ScriptData` and asserted equal.

use std::str::FromStr;

use pallas_addresses::Address as PallasAddress;
use pallas_crypto::hash::Hash;
use pallas_primitives::Fragment;
use pallas_primitives::conway::Tx;
use pallas_txbuilder::{BuildConway, Input, Output, StagingTransaction};

// ---------------------------------------------------------------------------
// Fixtures & helpers
// ---------------------------------------------------------------------------

/// A proven-valid mainnet base address (payment key + stake key), reused from
/// the `pallas-addresses` test corpus.
const ADDR_BASE: &str = "addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x";

/// Build a 32-byte hash whose every byte is `b` (for transaction ids etc.).
fn hash32(b: u8) -> Hash<32> {
    Hash::<32>::from([b; 32])
}

fn base_address() -> PallasAddress {
    PallasAddress::from_str(ADDR_BASE).expect("fixture address must be valid")
}

/// Decode the bytes of a built transaction back into a primitives `Tx`.
fn decode(tx_bytes: &[u8]) -> Tx<'_> {
    Tx::decode_fragment(tx_bytes).expect("built tx must decode as conway::Tx")
}

// ---------------------------------------------------------------------------
// Group A — minimal build
// ---------------------------------------------------------------------------

/// The simplest possible transaction: one input, one output, a fixed fee.
fn minimal_tx() -> StagingTransaction {
    StagingTransaction::new()
        .input(Input::new(hash32(0), 0))
        .output(Output::new(base_address(), 2_000_000))
        .fee(170_000)
}

#[test]
fn minimal_build_round_trips() {
    let built = minimal_tx().build_conway_raw().expect("build should succeed");

    let tx = decode(&built.tx_bytes.0);
    let body = &tx.transaction_body;

    assert_eq!(body.inputs.len(), 1, "exactly one input");
    assert_eq!(body.outputs.len(), 1, "exactly one output");
    assert_eq!(body.fee, 170_000, "fee preserved");
    assert!(body.ttl.is_none(), "no TTL was set");
    assert!(
        tx.transaction_witness_set.vkeywitness.is_none(),
        "unsigned tx has no vkey witnesses"
    );
}

#[test]
fn minimal_build_input_matches() {
    let built = minimal_tx().build_conway_raw().unwrap();
    let tx = decode(&built.tx_bytes.0);

    let input = tx.transaction_body.inputs.first().unwrap();
    assert_eq!(*input.transaction_id, [0u8; 32]);
    assert_eq!(input.index, 0);
}

#[test]
fn minimal_build_is_deterministic() {
    let a = minimal_tx().build_conway_raw().unwrap();
    let b = minimal_tx().build_conway_raw().unwrap();
    assert_eq!(a.tx_bytes.0, b.tx_bytes.0, "same inputs => same bytes");
    assert_eq!(a.tx_hash.0, b.tx_hash.0, "same inputs => same hash");
}

/// Golden snapshot: pins the exact CBOR and hash of the minimal transaction.
/// If these drift, the encoder output changed — review the diff, and if the
/// change is intended, regenerate the files under `tests/golden/`.
#[test]
fn minimal_build_golden() {
    let built = minimal_tx().build_conway_raw().unwrap();

    assert_eq!(
        hex::encode(&built.tx_bytes.0),
        include_str!("golden/minimal.tx").trim(),
        "minimal tx CBOR drifted",
    );
    assert_eq!(
        hex::encode(built.tx_hash.0),
        include_str!("golden/minimal.hash").trim(),
        "minimal tx hash drifted",
    );
}
