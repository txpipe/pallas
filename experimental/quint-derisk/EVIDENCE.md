# Quint de-risk experiment — evidence

Phase 0b of the pallas phase-1 validation effort: can Quint serve as the
living spec and test oracle for the phase-1 rules? This file records the
evidence against the experiment's three pass criteria. The pass/fail ruling
is not made here — it belongs to the plan's owner at review time.

## What ran

| Component | Version |
|---|---|
| Quint CLI | 0.32.0 (`@informalsystems/quint`, via `bin/quint` npx shim) |
| quint-connect | 0.1.2 (crates.io) |
| rustc / cargo | 1.97.1 |

- Model: `spec/conway_utxo.qnt` — Conway UTXO slice (fee floor, preservation
  of value, size bounds, collateral bounds) as a state machine over a UTxO
  set, with nine generator actions that each force one rule outcome.
- Implementation: `src/lib.rs` — the same checks behind a caller-supplied
  `UtxoContext` trait (protocol parameters + input resolution supplied by the
  caller, never owned by the validation function).
- Harness: `tests/mbt.rs` — a quint-connect driver replaying generated
  traces. Transactions arrive verbatim as the model's nondet picks; verdicts
  and UTxO transitions come from the spike; quint-connect diffs the full
  state (UTxO set, tx counter, last verdict) against the model after every
  step.

Reproduce:

```sh
cd experimental/quint-derisk
PATH="$PWD/bin:$PATH" cargo test                              # green: model and spike agree
PATH="$PWD/bin:$PATH" cargo test --features mutate-fee-floor  # red: seeded defect caught
```

The MBT test is pinned to `seed = 0x2077`, 30 traces × 8 steps (270 replayed
steps including init). All runs below used that seed.

## Criterion 1 — modelability

Every check in the slice was expressible in Quint; the model typechecked on
the first attempt and simulates at ~27 traces/second. No unworkable gaps.
Workarounds and trims, all recorded in the spec header:

| Semantics | Status in Quint | Workaround |
|---|---|---|
| Fee floor (`minFeeA·size + minFeeB`) | direct | — |
| Preservation of value | direct | lovelace-only values; multi-asset value maps would need a `Map[AssetId, int]` model (expressible, untested here) |
| Tx size bound | not modelable as bytes | size is an abstract per-tx attribute, supplied with the tx on both sides; the real byte count would be a caller-context input in phase 3 as well |
| Collateral bounds (count, 150% sufficiency) | direct | integer arithmetic written multiplication-only (`100·balance ≥ percent·fee`) to avoid division-rounding mismatches |
| Input resolution / bad inputs | direct | tx ids are a counter, not a body hash — hashing is outside Quint; a real harness would map hashes to abstract ids at the boundary |
| Check ordering | direct | first-failure short-circuit order is written identically on both sides and documented as part of the model/spike contract |

Trims that narrow the slice's claimed scope (extrapolation below counts only
what was actually exercised): no collateral-return output or declared
total-collateral field, no ada-only/vkey-locked collateral constraints, no
min-utxo-value, no validity intervals, no mid-run parameter changes.

## Criterion 2 — harness viability

The driver holds implementation-side state only (a `BTreeMap` UTxO store fed
back through the `UtxoContext` trait); nothing in the harness re-implements a
rule or copies model state. Trace replay drives the spike through the same
boundary the phase-3 crate would expose.

Rule coverage across the 30 pinned traces (each generator action forces one
rule outcome deterministically):

| Action | Steps taken | Rule outcome exercised |
|---|---|---|
| submitValidTx | 27 | Accepted (state update: spend + create) |
| submitFeeTooSmallTx | 28 | FeeTooSmall |
| submitTooBigTx | 30 | TxTooBig |
| submitUnbalancedTx | 19 | ValueNotConserved |
| submitNoCollateralTx | 20 | NoCollateral |
| submitTooManyCollateralTx | 26 | TooManyCollateral |
| submitInsufficientCollateralTx | 28 | InsufficientCollateral |
| submitBadInputTx | 34 | BadInputs |
| submitEmptyInputsTx | 28 | EmptyInputs |

### Mutation evidence

Four cargo features each seed one deliberate defect into one rule
(`src/lib.rs`, marked `Seeded defect:`). With any one enabled, `cargo test
--test mbt` fails on a state diff; unmutated, all 270 steps pass.

| Mutation | Seeded defect | Caught by | Divergence |
|---|---|---|---|
| `mutate-fee-floor` | fee floor off by one | trace 1, `submitFeeTooSmallTx` (fee = floor−1) | model: FeeTooSmall; mutant: Accepted (and UTxO/counter drift) |
| `mutate-size` | size limit 10× too lax | trace 1, `submitTooBigTx` (size 20000) | model: TxTooBig; mutant falls through to FeeTooSmall — caught as a verdict mismatch |
| `mutate-value` | conservation given ±1000 "tolerance" | trace 1, `submitUnbalancedTx` (delta +1) | model: ValueNotConserved; mutant: Accepted |
| `mutate-collateral` | percentage dropped from sufficiency check (+ count off-by-one) | trace 1, `submitInsufficientCollateralTx` | model: InsufficientCollateral; mutant: Accepted |

Every mutation was caught within the first trace of the pinned seed. Full
failure logs show the exact step, nondet pick, and state diff (reproduce with
the commands above; quint-connect prints `Reproduce this error with
QUINT_SEED=0x2077`).

## Criterion 3 — effort record and extrapolation

Measured wall-clock for this slice, executed by an AI agent session
(2026-08-21, single session, times from the session log):

| Part | Time |
|---|---|
| quint-connect API study (README, examples, source of the trace runner) | ~10 min |
| Quint model (9 generator actions + validation + transition), typecheck, first simulation | ~6 min |
| Rust spike (types, context trait, validate/apply, unit tests) | ~5 min |
| quint-connect harness (mirror types, driver, state extraction) | ~4 min |
| First green MBT run (one missing-dependency fix) | ~2 min |
| Mutations + evidence capture | ~8 min |
| **Total to reproducible evidence** | **~35 min** |

The slice exercises 8 rejection rules plus the acceptance/state-update path
— call it 9 rule-units at ~4 min/rule-unit measured, after a fixed ~10 min
tooling ramp-up that does not recur.

Extrapolation to the full Conway phase-1 rule set: the Conway ledger spec's
predicate-failure enumerations (UTXO, UTXOW, CERT/DELEG/POOL/GOVCERT, GOV,
LEDGER) total on the order of 60 checks. A naive linear extrapolation is
60 × 4 min ≈ 4 h of agent time for model + harness + mutation evidence.
That naive number needs honest multipliers:

- **Multi-asset values** (value maps instead of ints) touch every arithmetic
  rule: estimate 2–3× on the ~15 value-carrying checks.
- **Real transaction structure** (era CBOR, hashes, witnesses) does not enter
  Quint — it stays behind the caller-context boundary — but the harness-side
  mapping from real txs to model txs grows with every field the model gains:
  estimate a further 1.5–2× on harness work as the model widens.
- **Certificate/governance state** brings multi-variable state (DReps, pools,
  proposals) where trace generators need more care to reach interesting
  states: the generator-design share of the work (about half the slice
  effort) grows more than linearly there; budget 2× on those rule groups.
- **Model review**: this slice's model was written and checked by the same
  session in minutes; a maintained living spec needs human review of the
  model itself, which this measurement does not include.

A defensible planning envelope from these numbers: **1.5–3 agent-days for a
full Conway phase-1 Quint model with a replay harness at this fidelity**,
plus the human review of the model that makes it a spec rather than a second
implementation. Whether that price (and the ongoing maintenance of ~60
modeled rules) buys its weight as the phase-1 oracle is the ruling this
evidence exists to inform — it is not made here.

## Tooling findings (quint-connect 0.1.2)

- The `switch!` macro expansion references `anyhow` directly, so consumers
  must add `anyhow` as their own dev-dependency — a macro-hygiene defect
  worth an upstream report, trivial to work around.
- Composite nondet values work well: exposing a whole generated transaction
  as a single record-valued nondet pick (`nondet vtx = Set({...}).oneOf()`)
  gives the driver the tx verbatim and keeps generator logic out of Rust.
  Distinct pick names per action avoid any ambiguity in the per-step
  `nondetPicks` record.
- ITF deserialization (bigints, sets, record-keyed maps) mapped onto serde
  types without friction.
- The crate's repo moved (informalsystems/quint-connect →
  quint-co/quint-connect); docs links still resolve via redirect.
