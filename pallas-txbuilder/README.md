# Pallas TxBuilder

Ergonomic builder for constructing and signing Cardano transactions. The
crate is organised around `StagingTransaction`, a fluent builder that
collects inputs, outputs, mint, scripts, datums, and redeemers, and
finalises into a `BuiltTransaction` ready to be signed and submitted.

Currently the only era supported for building is **Conway** (via the
`BuildConway` trait). Earlier-era builders are intentionally not
maintained.

## Usage

```rust
use pallas_txbuilder::{BuildConway, Input, Output, StagingTransaction};

let tx = StagingTransaction::new()
    .input(Input::new(prev_tx_hash, 0))
    .output(Output::new(recipient_address, 2_000_000))
    .fee(170_000)
    .build_conway_raw()?;

let signed = tx.sign(&signing_key)?;
let cbor = signed.tx_bytes;
```

## Overview

- `StagingTransaction` — the in-progress, mutable transaction; the entry
  point for everything (`new`, `input`, `output`, `mint`, `fee`,
  `network_id`, `valid_after`, …).
- `BuiltTransaction` — the finalised, encoded body produced by `BuildConway`;
  exposes `sign(&signer)` and the raw CBOR bytes.
- `Input`, `Output`, `ExUnits`, `ScriptKind`, `Bytes`, `Bytes32` — the value
  types that go into the builder.
- `BuildConway` trait — implemented for `StagingTransaction`; turns staging
  state into a Conway-encoded transaction.
- `TxBuilderError` — the unified error returned from build / sign.
