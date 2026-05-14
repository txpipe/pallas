//! Phase-1 (and optionally phase-2) Cardano transaction validation against
//! the live ledger rules.
//!
//! Useful for clients that want to reject ill-formed or unenforceable
//! transactions locally before submitting them, or to replay historical
//! chains and confirm conformance with the protocol specification.
//!
//! # Usage
//!
//! ```ignore
//! use pallas_validate::phase1::validate_tx;
//!
//! validate_tx(&tx, tx_index, &env, &utxos, &mut cert_state)?;
//! ```
//!
//! [`phase1::validate_tx`] dispatches on the era encoded in the
//! `Environment.prot_params` and routes to the matching era-specific
//! validator ([`phase1::byron::validate_byron_tx`],
//! [`phase1::shelley_ma::validate_shelley_ma_tx`],
//! [`phase1::alonzo::validate_alonzo_tx`],
//! [`phase1::babbage::validate_babbage_tx`],
//! [`phase1::conway::validate_conway_tx`]).
//!
//! # Overview
//!
//! - [`phase1`] — phase-1 (structural / rule-based) validation, with one
//!   module per era: [`phase1::byron`], [`phase1::shelley_ma`],
//!   [`phase1::alonzo`], [`phase1::babbage`], [`phase1::conway`]. Top-level
//!   entry points are [`phase1::validate_tx`] (single tx) and
//!   [`phase1::validate_txs`] (LEDGERS sequence rule).
//! - `phase2` — phase-2 (Plutus script execution) validation. Behind the
//!   `phase2` cargo feature.
//! - [`utils`] — the shared input types every validator takes:
//!   [`utils::Environment`], [`utils::UTxOs`], [`utils::CertState`],
//!   [`utils::MultiEraProtocolParameters`], and the unified
//!   [`utils::ValidationError`] / [`utils::ValidationResult`] types.
//!
//! # Feature flags
//!
//! - `phase2` — pulls in Plutus script execution and exposes the `phase2`
//!   module.
//!
//! # Further reading
//!
//! - `docs/byron.md`, `docs/shelleyMA.md`, `docs/alonzo.md`,
//!   `docs/babbage.md` — mathematical specifications, one per era.
//! - `tests/README.md` — test-suite layout and how to reproduce the per-era
//!   fixtures.
//!
//! # Usage as part of `pallas`
//!
//! When depending on the umbrella [`pallas`] crate, this crate is re-exported
//! as `pallas::ledger::validate`.
//!
//! [`pallas`]: https://crates.io/crates/pallas

/// Phase-1 (structural / rule-based) validation, with one module per era.
pub mod phase1;
/// Shared input types: [`utils::Environment`], [`utils::UTxOs`],
/// [`utils::CertState`], [`utils::ValidationError`], and friends.
pub mod utils;

/// Phase-2 (Plutus script execution) validation (feature `phase2`).
#[cfg(feature = "phase2")]
pub mod phase2;
