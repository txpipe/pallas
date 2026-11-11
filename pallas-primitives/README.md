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

## Decode depth limit

`PlutusData`, `Metadatum` and the native script types refuse CBOR in which a
node would sit under `pallas_codec::MAX_DECODE_DEPTH` (16,384) enclosing
nodes, with a decode error. The error propagates, so a block or transaction
that contains such a value fails to decode as a whole. Each level takes at
least one byte, so at the current maximum transaction size of 16,384 bytes no
on-chain value reaches the limit.

Native scripts drop iteratively. Dropping `PlutusData` and `Metadatum` values
still recurses once per nesting level, and the stack that takes depends on
how the consuming binary compiles the drop code. In
release builds, a `PlutusData` value at the limit has needed more than the
2 MiB thread default in some builds, so give threads that drop deeply nested
untrusted values at least 4 MiB of stack, and more in debug builds. Derived
`Debug` and Serde also still recurse.
