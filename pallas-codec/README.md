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
