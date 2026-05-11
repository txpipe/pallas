//! Fixed-precision arithmetic for the parts of the Cardano protocol that
//! need `ln`, `exp`, and `pow` to high precision.
//!
//! The Praos VRF leader-check formula and the reward / pool-saturation
//! calculations both require deterministic transcendental math; this crate
//! supplies it. Backed by [`dashu`] for the underlying big-integer
//! representation.
//!
//! [`dashu`]: https://docs.rs/dashu
//!
//! # Usage
//!
//! ```
//! use pallas_math::math::{FixedDecimal, FixedPrecision};
//!
//! let x = FixedDecimal::from_str("2", 34)?;   // 2.0 with 34 fractional digits
//! let y = x.ln();                             // ≈ 0.6931471805599453…
//! println!("ln(2) ≈ {y}");
//! # Ok::<_, pallas_math::math::Error>(())
//! ```
//!
//! # Overview
//!
//! - [`math`] — the public surface: [`math::FixedDecimal`] (the
//!   fixed-precision number type) and the [`math::FixedPrecision`] trait it
//!   implements (`new`, `from_str`, `precision`, `exp`, `ln`, `pow`,
//!   `exp_cmp`, `round` / `floor` / `ceil` / `trunc`).
//! - [`math_dashu`] — `dashu`-backed implementation that
//!   [`math::FixedDecimal`] aliases.
//! - [`math::DEFAULT_PRECISION`] (34), and the lazy [`math::ZERO`],
//!   [`math::ONE`], [`math::MINUS_ONE`] constants for convenience.
//!
//! # Usage as part of `pallas`
//!
//! `pallas-math` is not currently re-exported from the umbrella `pallas`
//! crate; depend on it directly.

/// Public surface: the [`math::FixedDecimal`] type, the [`math::FixedPrecision`] trait,
/// and the shared constants.
pub mod math;
/// `dashu`-backed implementation of [`math::FixedPrecision`].
pub mod math_dashu;
