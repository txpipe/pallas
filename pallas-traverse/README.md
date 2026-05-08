# Pallas Traverse

A read-only, era-agnostic view over Cardano blocks and transactions. Where
`pallas-primitives` exposes the raw typed CBOR per era, this crate hides
the era split behind `MultiEra*` enums so a single piece of indexing or
analysis code can run against everything from Byron to Conway.

This is the read side of the ledger. For tx construction see
`pallas-txbuilder`; for ledger-rule validation see `pallas-validate`.

## Usage

```rust
use pallas_traverse::MultiEraBlock;

let block = MultiEraBlock::decode(&cbor_bytes)?;

println!("era={:?} slot={} hash={}", block.era(), block.slot(), block.hash());

for tx in block.txs() {
    for output in tx.outputs() {
        println!("  → {} lovelace", output.lovelace_amount());
    }
}
```

## Overview

- `MultiEraBlock`, `MultiEraTx`, `MultiEraHeader` — top-level entry points
  with `decode` / `decode_for_era` constructors.
- `MultiEraInput`, `MultiEraOutput`, `MultiEraValue`, `MultiEraAsset`,
  `MultiEraPolicyAssets` — per-piece views.
- `MultiEraCert`, `MultiEraRedeemer`, `MultiEraWithdrawals`,
  `MultiEraSigners`, `MultiEraMeta`, `MultiEraUpdate`, `MultiEraProposal`,
  `MultiEraGovAction` — the rest of the tx surface, normalised across eras.
- `Era` and `Feature` — discriminators for "which era is this" and "does
  this era support X" (multi-assets, smart contracts, CIP-1694, …).
- Trait-driven hashing: `ComputeHash<N>` and `OriginalHash<N>` give you a
  uniform way to take Blake2b digests of typed structures.
- Per-aspect submodules for deeper helpers: `block`, `tx`, `input`,
  `output`, `assets`, `value`, `cert`, `redeemers`, `witnesses`,
  `signers`, `hashes`, `fees`, `governance`, `time`, `header`, `meta`,
  `auxiliary`, `probe`, `size`, `withdrawals`, `wellknown`.
