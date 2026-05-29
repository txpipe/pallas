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

// ---------------------------------------------------------------------------
// Group F — cross-check against real cardano-cli transactions
// ---------------------------------------------------------------------------
//
// The fixtures under tests/vectors/ are unsigned Conway transactions produced
// by cardano-cli and supplied by the crate maintainer. cardano-cli emits
// plain/token outputs in the *legacy* `[address, value]` array form, whereas
// the builder always emits the *post-Alonzo* map form — both are valid per the
// Conway CDDL. We therefore compare semantic content (inputs, outputs, fee,
// validity, mint, native scripts) and additionally require any output that is
// post-Alonzo in *both* to be byte-identical.

use std::collections::BTreeMap;

use pallas_primitives::conway::{DatumOption, TransactionOutput};
use pallas_txbuilder::BuiltTransaction;

// Addresses and payloads lifted from the fixtures, kept as raw bytes so the
// reconstruction is explicit rather than echoing a decoded address back.
const ADDR_DATUM: &str = "00dd996ca1174aa2e32dbbad88046b440ff563a3cde0716a56865400c6b5c562bdedfb6d283af13b35a63556c0d4acc5ea01069f96e7975a6b";
const ADDR_CHANGE: &str = "006864e676b09a8e5a6d8fac297f7e7f6dd9e2fae27bdd5e7b2aac24e08a49fde31d74eb97199efb297c375da5f6876596198360af927cd37a";
const ADDR_ENTERPRISE: &str = "60f60849dabf8cca552078db2838a4f6569ee36ae2ec5ea954b907fe60";
const INLINE_DATUM: &str = "d8799fa145627974657358383439356362343666393066643538623434656363633939623136306561343138326566313764663434636237646561633438303430656261ff";
// `all [ invalid_before 44203 ]` — its Blake2b-224 hash is the mint policy id.
const NATIVE_SCRIPT: &str = "820181820419acab";

fn addr(hex_str: &str) -> PallasAddress {
    PallasAddress::from_bytes(&hex::decode(hex_str).unwrap()).unwrap()
}

fn hash32_hex(h: &str) -> Hash<32> {
    Hash::<32>::from(<[u8; 32]>::try_from(hex::decode(h).unwrap()).unwrap())
}

fn hash28_hex(h: &str) -> Hash<28> {
    Hash::<28>::from(<[u8; 28]>::try_from(hex::decode(h).unwrap()).unwrap())
}

/// Flatten a `Multiasset`-shaped map into a sorted `(policy, name, qty)` list,
/// converting the quantity through `f`. The policy/name key types are shared
/// across eras (`Hash<28>` / `Bytes`); only the coin type differs (alonzo
/// `u64`, conway `PositiveCoin` / `NonZeroInt`).
fn flatten<A, T, F>(
    ma: &BTreeMap<Hash<28>, BTreeMap<pallas_primitives::Bytes, A>>,
    f: F,
) -> Vec<(Vec<u8>, Vec<u8>, T)>
where
    A: Copy,
    T: Ord,
    F: Fn(A) -> T,
{
    let mut out = vec![];
    for (policy, names) in ma {
        for (name, amt) in names {
            out.push((policy.to_vec(), name.to_vec(), f(*amt)));
        }
    }
    out.sort();
    out
}

/// (address, coin, sorted assets, inline datum bytes) — format-independent.
type OutputSummary = (Vec<u8>, u64, Vec<(Vec<u8>, Vec<u8>, u64)>, Option<Vec<u8>>);

fn summarize(out: &TransactionOutput) -> OutputSummary {
    use pallas_primitives::{alonzo, conway};
    match out {
        TransactionOutput::PostAlonzo(o) => {
            let (coin, assets) = match &o.value {
                conway::Value::Coin(c) => (*c, vec![]),
                conway::Value::Multiasset(c, ma) => (*c, flatten(ma, u64::from)),
            };
            let datum = match o.datum_option.as_deref() {
                Some(DatumOption::Data(d)) => Some(d.0.raw_cbor().to_vec()),
                _ => None,
            };
            (o.address.to_vec(), coin, assets, datum)
        }
        TransactionOutput::Legacy(o) => {
            let (coin, assets) = match &o.amount {
                alonzo::Value::Coin(c) => (*c, vec![]),
                alonzo::Value::Multiasset(c, ma) => (*c, flatten(ma, |v| v)),
            };
            (o.address.to_vec(), coin, assets, None)
        }
    }
}

fn inputs_of(tx: &Tx) -> Vec<([u8; 32], u64)> {
    let mut v: Vec<_> = tx
        .transaction_body
        .inputs
        .iter()
        .map(|i| (*i.transaction_id, i.index))
        .collect();
    v.sort();
    v
}

fn mint_of(tx: &Tx) -> Vec<(Vec<u8>, Vec<u8>, i64)> {
    tx.transaction_body
        .mint
        .as_ref()
        .map(|m| flatten(m, i64::from))
        .unwrap_or_default()
}

fn native_scripts_of(tx: &Tx) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = tx
        .transaction_witness_set
        .native_script
        .iter()
        .flat_map(|set| set.iter())
        .map(|s| s.raw_cbor().to_vec())
        .collect();
    v.sort();
    v
}

/// Decode `real_hex` and assert the builder's `built` is the same transaction:
/// equal inputs, fee, validity bounds, mint, native scripts, and per-output
/// content; plus byte-identical encoding for outputs that are post-Alonzo in
/// both (the legacy/post-Alonzo difference is a cardano-cli choice, not ours).
fn cross_check(real_hex: &str, built: &BuiltTransaction) {
    let real_bytes = hex::decode(real_hex.trim()).unwrap();
    let real = decode(&real_bytes);
    let got = decode(&built.tx_bytes.0);
    let (rb, gb) = (&real.transaction_body, &got.transaction_body);

    assert_eq!(inputs_of(&real), inputs_of(&got), "inputs");
    assert_eq!(rb.fee, gb.fee, "fee");
    assert_eq!(rb.ttl, gb.ttl, "ttl");
    assert_eq!(
        rb.validity_interval_start, gb.validity_interval_start,
        "validity start"
    );
    assert_eq!(mint_of(&real), mint_of(&got), "mint");
    assert_eq!(native_scripts_of(&real), native_scripts_of(&got), "scripts");

    assert_eq!(rb.outputs.len(), gb.outputs.len(), "output count");
    for (i, (r, g)) in rb.outputs.iter().zip(gb.outputs.iter()).enumerate() {
        assert_eq!(summarize(r), summarize(g), "output {i} content");
        if let (TransactionOutput::PostAlonzo(ro), TransactionOutput::PostAlonzo(go)) = (r, g) {
            assert_eq!(ro.raw_cbor(), go.raw_cbor(), "post-alonzo output {i} bytes");
        }
    }
}

/// A real datum-bearing payment: one input, an inline-datum output, change.
#[test]
fn reconstructs_real_datum_tx() {
    let built = StagingTransaction::new()
        .input(Input::new(
            hash32_hex("d3c742ca3c01719349ff3d969651e78802f93ebae48fc9c43ca247374e372aa6"),
            1,
        ))
        .output(
            Output::new(addr(ADDR_DATUM), 1_305_930)
                .set_inline_datum(hex::decode(INLINE_DATUM).unwrap()),
        )
        .output(Output::new(addr(ADDR_CHANGE), 6_787_093_941))
        .fee(172_101)
        .invalid_from_slot(124_418_399)
        .valid_from_slot(1)
        .build_conway_raw()
        .unwrap();

    cross_check(include_str!("vectors/datum_output.tx"), &built);
}

/// A real token transfer: two inputs, an inline-datum + token output, plain
/// change, and token change.
#[test]
fn reconstructs_real_token_tx() {
    let policy = hash28_hex("c35ca45f0a48f857063665fdd83b8703110fce2abb88188760c0c2ef");
    let name =
        hex::decode("001bc280021c22239e3a0d64bdb158e1563b2ac4b8be3a2b5d89c7ef7104790d").unwrap();

    let built = StagingTransaction::new()
        .input(Input::new(
            hash32_hex("d3c742ca3c01719349ff3d969651e78802f93ebae48fc9c43ca247374e372aa6"),
            1,
        ))
        .input(Input::new(
            hash32_hex("df4f3e2f340764a40834d3a54d32a159f97fcdacc4d780b860e7b626b80fcf2d"),
            2,
        ))
        .output(
            Output::new(addr(ADDR_DATUM), 1_599_010)
                .add_asset(policy, name.clone(), 12)
                .unwrap()
                .set_inline_datum(hex::decode(INLINE_DATUM).unwrap()),
        )
        .output(Output::new(addr(ADDR_CHANGE), 6_786_790_345))
        .output(
            Output::new(addr(ADDR_CHANGE), 2_706_249)
                .add_asset(policy, name, 605)
                .unwrap(),
        )
        .fee(182_617)
        .invalid_from_slot(124_418_634)
        .valid_from_slot(1)
        .build_conway_raw()
        .unwrap();

    cross_check(include_str!("vectors/token_output.tx"), &built);
}

/// A real native-script mint: two inputs, a token output + plain change, a
/// mint of the token, and the native-script policy in the witness set.
#[test]
fn reconstructs_real_native_mint_tx() {
    let policy = hash28_hex("c65cb5e0a28be0fc30cef5c53f55bc665740062e1e24f65b7d310d21");
    let name = hex::decode("74537461626c65").unwrap();

    let built = StagingTransaction::new()
        .input(Input::new(
            hash32_hex("7553d538a0a00d1c6362083d1c2d262e78252993cec88631794273141c267c94"),
            0,
        ))
        .input(Input::new(
            hash32_hex("7553d538a0a00d1c6362083d1c2d262e78252993cec88631794273141c267c94"),
            2,
        ))
        .output(
            Output::new(addr(ADDR_DATUM), 1_168_010)
                .add_asset(policy, name.clone(), 123_456_789)
                .unwrap(),
        )
        .output(Output::new(addr(ADDR_ENTERPRISE), 37_061_345))
        .mint_asset(policy, name, 123_456_789)
        .unwrap()
        .script(ScriptKind::Native, hex::decode(NATIVE_SCRIPT).unwrap())
        .fee(182_485)
        .invalid_from_slot(124_408_635)
        .valid_from_slot(124_408_034)
        .build_conway_raw()
        .unwrap();

    cross_check(include_str!("vectors/native_mint.tx"), &built);
}
