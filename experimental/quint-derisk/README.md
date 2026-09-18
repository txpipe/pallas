# Quint de-risk spike (EXPERIMENT)

Phase 0b of the pallas phase-1 validation effort: a de-risk experiment
measuring whether a Quint executable spec, replayed against Rust with
[quint-connect](https://github.com/quint-co/quint-connect), can serve as the
living spec and test oracle for the phase-1 ledger rules.

**This is experiment evidence, not product code.** The crate is excluded
from the pallas workspace (its own `[workspace]` root), never published, and
is *not* the phase-1 validation implementation — that will be a fresh start
under the phase-1 design. Whether this directory merges, is archived, or is
deleted is part of the experiment's ruling.

## Contents

- `spec/conway_utxo.qnt` — Quint model of a Conway UTXO slice: fee floor,
  preservation of value, size bounds, collateral bounds. The model is the
  oracle: it validates and applies every generated transaction.
- `src/lib.rs` — the same checks in Rust, behind a caller-supplied
  `UtxoContext` boundary. Includes four `mutate-*` cargo features that each
  seed one deliberate defect for mutation testing.
- `tests/mbt.rs` — quint-connect driver replaying model traces against the
  spike, diffing full state every step.
- `EVIDENCE.md` — the experiment record: modelability notes, rule coverage,
  mutation matrix, effort measurement and extrapolation.
- `bin/quint` — npx shim pinning the Quint CLI version.

## Running

Requires Rust and Node (for the Quint CLI via npx):

```sh
cd experimental/quint-derisk
PATH="$PWD/bin:$PATH" cargo test --locked                              # model and spike agree
PATH="$PWD/bin:$PATH" cargo test --locked --features mutate-fee-floor  # seeded defect: test fails
PATH="$PWD/bin:$PATH" QUINT_VERBOSE=1 cargo test --locked -- --nocapture  # show every step
```

Standalone model checks:

```sh
bin/quint typecheck spec/conway_utxo.qnt
bin/quint run spec/conway_utxo.qnt --invariant nonNegativeUtxo --mbt
```
