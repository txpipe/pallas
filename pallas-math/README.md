# Pallas Math

Fixed-precision arithmetic for the parts of the Cardano protocol that need
to compute `ln`, `exp`, and `pow` to high precision — most notably the
reward and pool-saturation calculations. Backed by [`dashu`] for the
underlying big-integer representation.

[`dashu`]: https://docs.rs/dashu

## Usage

```rust
use pallas_math::math::{FixedDecimal, FixedPrecision};

let x = FixedDecimal::from_str("2", 34)?;   // 2.0 with 34 fractional digits
let y = x.ln();                             // ≈ 0.6931471805599453…
println!("ln(2) ≈ {y}");
```

## Overview

- `math` — the public surface: `FixedDecimal` (the fixed-precision number
  type) and the `FixedPrecision` trait it implements (`new`, `from_str`,
  `precision`, `exp`, `ln`, `pow`, `exp_cmp`, `round`/`floor`/`ceil`/`trunc`).
- `math_dashu` — `dashu`-backed implementation that `FixedDecimal` aliases.
- `DEFAULT_PRECISION` (34), and the lazy `ZERO` / `ONE` / `MINUS_ONE`
  constants for convenience.

## Consensus arithmetic

At E34 precision, signed multiplication and division round toward negative
infinity, matching the pinned Cardano-node `Data.Fixed` arithmetic. Exponential
comparison uses node ABOVE/BELOW bounds: `GT` means ABOVE, `LT` means BELOW, and
`UNKNOWN` means the supplied iteration budget was exhausted.

The offline tests retain the original C corpus byte-for-byte and compare
numerical values and election comparisons with pinned node-oracle fixtures.
Recorded C numerical differences are checked against `oracle-divergences.json`;
C comparator approximations and iteration counts are not consensus authorities.

The fixtures were generated and independently checked with an external Haskell
oracle using `non-integral` 1.0.0.0, `cardano-ledger-core` 1.21.0.0,
`cardano-protocol` 0.1.0.0, `cardano-crypto-class` 2.5.1.0 and
`cardano-crypto-praos` 2.2.4.0. Generation used Hackage index state
`2026-08-14T13:38:07Z` and CHaP index state `2026-09-23T18:57:23Z`.
This checkout contains the offline test fixtures, not the oracle source,
generation scripts or Cabal project/freeze used to produce them. Rust tests do
not require Haskell tooling.
