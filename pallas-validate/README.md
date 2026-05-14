# Pallas Validate

Phase-1 (and optionally phase-2) Cardano transaction validation against
the live ledger rules. Useful for clients that want to reject
ill-formed or unenforceable transactions locally before submitting them,
or to replay historical chains and confirm conformance with the protocol
specification.

## Usage

```rust
use pallas_validate::phase1::validate_tx;

validate_tx(&tx, tx_index, &env, &utxos, &mut cert_state)?;
```

`validate_tx` dispatches on the era encoded in `Environment.prot_params`
and routes to the matching era-specific validator (`validate_byron_tx`,
`validate_shelley_ma_tx`, `validate_alonzo_tx`, `validate_babbage_tx`,
`validate_conway_tx`).

## Overview

- `phase1` — phase-1 (structural / rule-based) validation, with one module
  per era: `byron`, `shelley_ma`, `alonzo`, `babbage`, `conway`. Top-level
  entry points are `validate_tx` (single tx) and `validate_txs`
  (LEDGERS sequence rule).
- `phase2` — phase-2 (Plutus script execution) validation. Behind the
  `phase2` cargo feature.
- `utils` — the shared input types every validator takes:
  `Environment`, `UTxOs`, `CertState`, `MultiEraProtocolParameters`, and
  the unified `ValidationError` / `ValidationResult` types.

## Feature flags

- `phase2` — pulls in Plutus script execution and exposes the `phase2`
  module.

## Further reading

- [`docs/byron.md`](docs/byron.md), [`docs/shelleyMA.md`](docs/shelleyMA.md),
  [`docs/alonzo.md`](docs/alonzo.md), [`docs/babbage.md`](docs/babbage.md)
  — mathematical specifications, one per era.
- [`tests/README.md`](tests/README.md) — test-suite layout and how to
  reproduce the per-era fixtures.
