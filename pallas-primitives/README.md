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
