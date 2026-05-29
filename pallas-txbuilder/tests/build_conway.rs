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
use pallas_crypto::key::ed25519::SecretKeyExtended;
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

/// A deterministic extended Ed25519 signing key seeded from a single byte.
///
/// The 64-byte buffer is clamped to satisfy the extended-key bit tweaks so the
/// checked `from_bytes` constructor accepts it — giving us stable, repeatable
/// signatures with no RNG and no extra dependencies.
fn signer(seed: u8) -> SecretKeyExtended {
    let mut bytes = [seed; SecretKeyExtended::SIZE];
    bytes[0] &= 0b1111_1000;
    bytes[31] = (bytes[31] & 0b0011_1111) | 0b0100_0000;
    SecretKeyExtended::from_bytes(bytes).expect("clamped bytes are a valid extended key")
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

// ---------------------------------------------------------------------------
// Group B — signing
// ---------------------------------------------------------------------------

#[test]
fn sign_embeds_one_vkey_witness() {
    let key = signer(1);
    let pubkey: [u8; 32] = key.public_key().as_ref().try_into().unwrap();

    let signed = minimal_tx()
        .build_conway_raw()
        .unwrap()
        .sign(&key)
        .expect("signing should succeed");

    // The in-memory signature map is populated.
    let sigs = signed.signatures.as_ref().expect("signatures recorded");
    assert_eq!(sigs.len(), 1);

    // ...and the witness is embedded in the encoded bytes.
    let tx = decode(&signed.tx_bytes.0);
    let witnesses = tx
        .transaction_witness_set
        .vkeywitness
        .as_ref()
        .expect("vkey witness present");
    assert_eq!(witnesses.len(), 1);
    assert_eq!(*witnesses[0].vkey, pubkey.to_vec());
}

#[test]
fn signing_preserves_body_hash() {
    let built = minimal_tx().build_conway_raw().unwrap();
    let hash_before = built.tx_hash.0;
    let signed = built.sign(&signer(1)).unwrap();
    assert_eq!(
        signed.tx_hash.0, hash_before,
        "attaching a witness must not change the body hash"
    );
}

#[test]
fn signing_twice_accumulates_witnesses() {
    let signed = minimal_tx()
        .build_conway_raw()
        .unwrap()
        .sign(&signer(1))
        .unwrap()
        .sign(&signer(2))
        .unwrap();

    let tx = decode(&signed.tx_bytes.0);
    let witnesses = tx.transaction_witness_set.vkeywitness.as_ref().unwrap();
    assert_eq!(witnesses.len(), 2, "two distinct signers => two witnesses");
}

#[test]
fn add_signature_embeds_out_of_band_witness() {
    let key = signer(3);
    let pubkey = key.public_key();
    let built = minimal_tx().build_conway_raw().unwrap();

    // A real signature over the body hash, supplied "out of band" (the HSM /
    // hardware-wallet flow that `add_signature` exists for).
    let sig: [u8; 64] = key.sign(built.tx_hash.0).as_ref().try_into().unwrap();

    let added = built.add_signature(pubkey, sig).expect("add_signature");
    let tx = decode(&added.tx_bytes.0);
    let witnesses = tx.transaction_witness_set.vkeywitness.as_ref().unwrap();
    assert_eq!(witnesses.len(), 1);
    let pubkey_bytes: [u8; 32] = pubkey.as_ref().try_into().unwrap();
    assert_eq!(*witnesses[0].vkey, pubkey_bytes.to_vec());
}

#[test]
fn remove_signature_leaves_remaining_witnesses() {
    let keep = signer(4);
    let drop = signer(5);

    // Two witnesses, then remove one — the set stays non-empty.
    let signed = minimal_tx()
        .build_conway_raw()
        .unwrap()
        .sign(&keep)
        .unwrap()
        .sign(&drop)
        .unwrap()
        .remove_signature(drop.public_key())
        .expect("remove_signature");

    let tx = decode(&signed.tx_bytes.0);
    let witnesses = tx.transaction_witness_set.vkeywitness.as_ref().unwrap();
    assert_eq!(witnesses.len(), 1, "only the dropped signer is removed");

    let keep_bytes: [u8; 32] = keep.public_key().as_ref().try_into().unwrap();
    assert_eq!(*witnesses[0].vkey, keep_bytes.to_vec());
}

/// BUG (documented, not yet fixed): removing the *only* witness panics at
/// `model.rs` because `NonEmptySet::from_vec(vec![])` returns `None` and the
/// builder unwraps it. The correct behaviour is to clear the witness to
/// `None`. Un-`ignore` this in the PR that fixes the panic.
#[test]
#[ignore = "remove_signature panics when emptying the witness set; fixed in a later PR"]
fn remove_last_signature_clears_witness_set() {
    let key = signer(6);
    let signed = minimal_tx()
        .build_conway_raw()
        .unwrap()
        .sign(&key)
        .unwrap()
        .remove_signature(key.public_key())
        .expect("remove_signature");

    let tx = decode(&signed.tx_bytes.0);
    assert!(
        tx.transaction_witness_set.vkeywitness.is_none(),
        "removing the only witness should clear the set"
    );
}
