# Pallas Codec

The encoding foundation that the rest of the workspace builds on:
[minicbor](https://docs.rs/minicbor) for CBOR (re-exported) and a Rust port
of the Plutus Core [flat] format. Most users won't depend on this crate
directly — they'll get its types transitively through `pallas-primitives`,
`pallas-traverse`, `pallas-txbuilder`, etc.

[flat]: https://github.com/Quid2/flat

## Usage

```rust
use pallas_codec::minicbor;

#[derive(minicbor::Encode, minicbor::Decode)]
struct Pair(#[n(0)] u64, #[n(1)] String);

let bytes = minicbor::to_vec(Pair(1, "hi".into()))?;
let back: Pair = minicbor::decode(&bytes)?;
```

## Overview

- `minicbor` — re-exported as-is; this is the workspace's single source of
  truth for CBOR.
- `flat` — Rust port of the Haskell [flat] reference implementation, used
  for Plutus Core scripts.
- `utils` — round-trip-friendly helper types (`KeepRaw`, `KeyValuePairs`,
  `MaybeIndefArray`, `NonEmptySet`, `Nullable`, `PositiveCoin`, …) reused
  by the higher-level era types.
- `Fragment` trait — blanket-implemented for any type that is both
  `minicbor::Encode` and `minicbor::Decode`; used as a bound where the
  workspace wants "any CBOR-roundtrippable type".
- `codec_by_datatype!` macro — derives a tag-free CBOR codec for enums
  whose variants are distinguished by their data-type rather than a
  discriminant.

## Decode depth limit

`tree::decode_tree` decodes recursive types without call-stack recursion and
accepts any nesting depth. `tree::decode_tree_with_max_depth` takes a depth
limit and returns a decode error once a node would sit under that many
enclosing nodes (the root is always decoded). Pallas's own recursive types
(`PlutusData`, `Metadatum`, native scripts) decode through it with
`MAX_DECODE_DEPTH` (16,384). The error propagates, so a block or transaction
that contains such a value fails to decode as a whole. Each level takes at
least one byte and a mainnet transaction is at most 16,384 bytes at the
current `maxTxSize`, so no on-chain value is refused.

Native scripts drop iteratively. Dropping a decoded `PlutusData` or
`Metadatum` value still recurses once per level, so the limit bounds the stack
a drop needs rather than removing the need. How much stack that is
depends on how the consuming binary compiles the drop code, so it varies with
the type, the compiler and the surrounding code. In release builds, a value at
the limit has needed anything from about 1 MiB to more than 2 MiB, the
`std::thread` and tokio worker default. Threads that drop deeply nested
untrusted values should have at least 4 MiB of stack in release builds, and
more in debug builds. Derived `Debug` and Serde also still recurse.
