//! Bech32 conventions for Cardano.
//!
//! Two CIP-defined surfaces:
//!
//! - [CIP-5] — human-readable prefixes for keys, hashes, and addresses
//!   (`addr`, `stake`, `pool`, `vrf_vk`, …).
//! - [CIP-14] — Blake2b-160 asset fingerprints (`asset1…`) for
//!   `(policy id, asset name)` pairs.
//!
//! [CIP-5]: https://cips.cardano.org/cips/cip5/
//! [CIP-14]: https://cips.cardano.org/cips/cip14/
//!
//! # Usage
//!
//! ```
//! use pallas_bech32::cip14::AssetFingerprint;
//!
//! let fp = AssetFingerprint::from_parts(
//!     "7eae28af2208be856f7a119668ae52a49b73725e326dc16579dcc373",
//!     "",
//! )?;
//!
//! assert_eq!(fp.finger_print()?, "asset1rjklcrnsdzqp65wjgrg55sy9723kw09mlgvlc3");
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
//!
//! # Overview
//!
//! - [`cip5`] — `KEYS`, `HASHES`, and `MISCELLANEOUS` constants holding the
//!   bech32 HRPs assigned by CIP-5.
//! - [`cip14`] — [`cip14::AssetFingerprint`] builds and prints the
//!   `asset1…` fingerprint for a `(policy id, asset name)` pair.

/// CIP-14 asset fingerprints.
pub mod cip14;
/// CIP-5 bech32 prefix constants.
pub mod cip5;
