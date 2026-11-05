# Pallas Primitives

Era-aware Cardano ledger types with their CBOR codecs. This is the data
layer that the rest of the Pallas ledger crates sit on: `pallas-traverse`
gives you a multi-era read API over these types, `pallas-validate`
applies ledger rules to them, and `pallas-txbuilder` builds new ones.

If you need raw, era-specific access to a `Tx`, `Block`, or `PlutusData`,
you want this crate. If you'd rather work over many eras through one
interface, reach for `pallas-traverse`.

## Usage

```rust
use pallas_codec::minicbor;
use pallas_primitives::conway;

let tx: conway::Tx = minicbor::decode(&cbor_bytes)?;

for input in tx.transaction_body.inputs.iter() {
    println!("{:?}#{}", input.transaction_id, input.index);
}
```

## Overview

- `byron`, `alonzo`, `babbage`, `conway` — one module per era, each
  exposing the era's `Block`, `Tx`, `TransactionInput`, `TransactionOutput`,
  `Value`, `Certificate`, `Metadata`, witness sets, and so on.
- `plutus_data` — re-exported `PlutusData`, `BigInt`, and helpers shared
  across eras.
- `framework` — common type aliases and codec primitives (`AddrKeyhash`,
  `Coin`, `PolicyId`, `RationalNumber`, `StakeCredential`,
  `TransactionInput`, `ExUnits`, `PlutusScript<V>`, …).
- Re-exports from `pallas-codec` (`Bytes`, `KeepRaw`, `KeyValuePairs`,
  `NonEmptySet`, `Set`, `Nullable`, …) and `pallas-crypto` (`Hash`).

## Ownership migration in 1.5

`Metadatum` and `PlutusData` now destroy deeply nested trees iteratively, in
both debug and release builds. This prevents stack exhaustion during normal
scope exit and decode-error cleanup.

This safety fix is released as a **minor version by maintainer decision**, but
it is a source compatibility break: Rust prohibits moving non-`Copy` payloads
out of an enum that implements `Drop`. Existing borrowed matches, constructors,
whole-value moves and CBOR/Serde representations continue to work.

Use a borrowed match for inspection. To consume a payload without cloning it,
use the new `into_*` accessors; a mismatched variant returns the original value
in `Err`:

```rust
use pallas_primitives::Metadatum;

let datum = Metadatum::Text("hello".into());
let text = datum.into_text().unwrap();
assert_eq!(text, "hello");
```

Metadata provides `into_bytes`, `into_text`, `into_array`, and `into_map`.
Plutus data provides `into_constr`, `into_map`, `into_array`, `into_bigint`, and
`into_bounded_bytes`. Matching through `&mut` and using `mem::take` or
`mem::replace` is also supported.

Destruction uses a heap-backed work list and does not impose a CBOR nesting
limit. Derived `Debug` and Serde operations still recurse; their stack safety
is outside this change.
