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

/// Build a 28-byte hash whose every byte is `b` (for policy ids etc.).
fn hash28(b: u8) -> Hash<28> {
    Hash::<28>::from([b; 28])
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
    let built = minimal_tx()
        .build_conway_raw()
        .expect("build should succeed");

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

// ---------------------------------------------------------------------------
// Group C — determinism
// ---------------------------------------------------------------------------
//
// Mint and output-asset bundles encode through `BTreeMap`-backed `Multiasset`,
// so their order is normalised and the output is deterministic regardless of
// the order assets were added. Redeemers, datums, and scripts are collected in
// `HashMap` iteration order and are *not* normalised — see the #[ignore]d test.

#[test]
fn multi_asset_output_is_deterministic() {
    let build = || {
        StagingTransaction::new()
            .input(Input::new(hash32(0), 0))
            .output(
                Output::new(base_address(), 2_000_000)
                    .add_asset(hash28(0xaa), b"TOKEN_A".to_vec(), 5)
                    .unwrap()
                    .add_asset(hash28(0xbb), b"TOKEN_B".to_vec(), 7)
                    .unwrap(),
            )
            .fee(180_000)
            .build_conway_raw()
            .unwrap()
    };
    assert_eq!(build().tx_bytes.0, build().tx_bytes.0);
}

#[test]
fn mint_is_deterministic() {
    let build = || {
        StagingTransaction::new()
            .input(Input::new(hash32(0), 0))
            .output(Output::new(base_address(), 2_000_000))
            .mint_asset(hash28(0xaa), b"A".to_vec(), 1)
            .unwrap()
            .mint_asset(hash28(0xbb), b"B".to_vec(), -1)
            .unwrap()
            .fee(180_000)
            .build_conway_raw()
            .unwrap()
    };
    assert_eq!(build().tx_bytes.0, build().tx_bytes.0);
}

/// BUG (documented, not yet fixed): datums are stored in a `HashMap` and
/// emitted in iteration order, so a transaction with multiple datums can
/// encode to different bytes (and a different hash) from one build to the
/// next. Each iteration builds the staging tx *fresh* (a fresh `HashMap` is
/// seeded differently), which is exactly how callers hit this. Un-`ignore`
/// this once datums/redeemers/scripts are sorted before encoding.
#[test]
#[ignore = "datum/redeemer/script ordering is non-deterministic; fixed in a later PR"]
fn multiple_datums_build_deterministically() {
    // Three distinct, valid PlutusData datums (bare CBOR integers).
    let build = || {
        StagingTransaction::new()
            .input(Input::new(hash32(0), 0))
            .output(Output::new(base_address(), 2_000_000))
            .fee(180_000)
            .datum(vec![0x01])
            .datum(vec![0x02])
            .datum(vec![0x03])
            .build_conway_raw()
            .unwrap()
            .tx_bytes
            .0
    };

    let encodings: std::collections::HashSet<Vec<u8>> = (0..64).map(|_| build()).collect();

    assert_eq!(
        encodings.len(),
        1,
        "the same datums must always encode identically"
    );
}

// ---------------------------------------------------------------------------
// Group D — multi-asset output and mint values
// ---------------------------------------------------------------------------

/// One output carrying lovelace + a native token, plus a mint and a burn.
fn asset_tx() -> StagingTransaction {
    StagingTransaction::new()
        .input(Input::new(hash32(0), 0))
        .output(
            Output::new(base_address(), 2_000_000)
                .add_asset(hash28(0xaa), b"TOKEN".to_vec(), 42)
                .unwrap(),
        )
        .mint_asset(hash28(0xaa), b"TOKEN".to_vec(), 42)
        .unwrap()
        .mint_asset(hash28(0xcc), b"OLD".to_vec(), -3)
        .unwrap()
        .fee(190_000)
}

#[test]
fn output_asset_value_round_trips() {
    use pallas_primitives::conway::{TransactionOutput, Value};

    let built = asset_tx().build_conway_raw().unwrap();
    let tx = decode(&built.tx_bytes.0);

    let TransactionOutput::PostAlonzo(out) = tx.transaction_body.outputs.first().unwrap() else {
        panic!("expected a post-alonzo output");
    };
    let Value::Multiasset(coin, assets) = &out.value else {
        panic!("expected a multiasset value");
    };

    assert_eq!(*coin, 2_000_000);
    let policy = assets.iter().find(|(p, _)| **p == hash28(0xaa)).unwrap();
    let (name, amount) = policy.1.iter().next().unwrap();
    assert_eq!(name.as_slice(), b"TOKEN");
    assert_eq!(u64::from(amount), 42);
}

#[test]
fn mint_and_burn_round_trip() {
    let built = asset_tx().build_conway_raw().unwrap();
    let tx = decode(&built.tx_bytes.0);

    let mint = tx.transaction_body.mint.as_ref().expect("mint present");
    let minted = mint.iter().find(|(p, _)| **p == hash28(0xaa)).unwrap();
    assert_eq!(
        i64::from(minted.1.iter().next().unwrap().1),
        42,
        "mint of 42"
    );

    let burned = mint.iter().find(|(p, _)| **p == hash28(0xcc)).unwrap();
    assert_eq!(
        i64::from(burned.1.iter().next().unwrap().1),
        -3,
        "burn of 3"
    );
}

/// Golden snapshot for the multi-asset + mint transaction.
#[test]
fn asset_tx_golden() {
    let built = asset_tx().build_conway_raw().unwrap();

    assert_eq!(
        hex::encode(&built.tx_bytes.0),
        include_str!("golden/multi_asset.tx").trim(),
        "multi-asset tx CBOR drifted",
    );
    assert_eq!(
        hex::encode(built.tx_hash.0),
        include_str!("golden/multi_asset.hash").trim(),
        "multi-asset tx hash drifted",
    );
}

// ---------------------------------------------------------------------------
// Group E — script-data hash self-consistency
// ---------------------------------------------------------------------------

use pallas_primitives::conway::{LanguageViews, ScriptData};
use pallas_txbuilder::{ExUnits, ScriptKind};

/// When redeemers, datums, and cost models are all present, the
/// `script_data_hash` the builder embeds must equal the value recomputed from
/// the decoded witness set — the same check `pallas-primitives` runs against
/// real on-chain transactions.
#[test]
fn script_data_hash_matches_recomputation() {
    let cost_model: Vec<i64> = vec![1, 2, 3, 4];

    let built = StagingTransaction::new()
        .input(Input::new(hash32(0), 0))
        .output(Output::new(base_address(), 2_000_000))
        .fee(200_000)
        .datum(vec![0x01])
        .add_spend_redeemer(
            Input::new(hash32(0), 0),
            vec![0x09],
            Some(ExUnits {
                mem: 1_000,
                steps: 500_000,
            }),
        )
        .add_language(ScriptKind::PlutusV3, cost_model.clone())
        .build_conway_raw()
        .unwrap();

    let tx = decode(&built.tx_bytes.0);
    let embedded = tx
        .transaction_body
        .script_data_hash
        .expect("script_data_hash must be set when redeemers/datums exist");

    let views = LanguageViews::from_iter([(2u8, cost_model)]);
    let recomputed = ScriptData::build_for(&tx.transaction_witness_set, &Some(views))
        .expect("witness has redeemers/datums")
        .hash();

    assert_eq!(embedded, recomputed, "embedded script_data_hash is wrong");
}

/// BUG (documented, not yet fixed): the builder computes `script_data_hash`
/// only when `language_views` is set, but the ledger requires it whenever the
/// witness set carries datums or redeemers. A datum-only transaction (no
/// redeemers, no cost models) therefore gets `script_data_hash = None` and is
/// rejected by the node. `pallas_primitives::ScriptData::build_for` already
/// handles this case; the builder should use it. Un-`ignore` once it does.
#[test]
#[ignore = "datum-only script_data_hash is omitted (gated on language_views); fixed in a later PR"]
fn datum_only_sets_script_data_hash() {
    let built = StagingTransaction::new()
        .input(Input::new(hash32(0), 0))
        .output(Output::new(base_address(), 2_000_000))
        .fee(200_000)
        .datum(vec![0x01])
        .build_conway_raw()
        .unwrap();

    let tx = decode(&built.tx_bytes.0);
    let embedded = tx.transaction_body.script_data_hash;

    // The correct value, per primitives, for a datum-only witness set.
    let expected = ScriptData::build_for(&tx.transaction_witness_set, &None)
        .expect("witness has datums")
        .hash();

    assert_eq!(
        embedded,
        Some(expected),
        "datum-only tx must still carry a script_data_hash"
    );
}
