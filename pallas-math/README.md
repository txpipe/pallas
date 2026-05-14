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
