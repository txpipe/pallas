//! Strongly typed parsers for Cardano genesis files and protocol parameters.
//!
//! One module per era exposes the canonical genesis shape and a `from_file`
//! helper, so tools that need to reason about staking metadata, cost models,
//! or other configuration data don't have to hand-roll JSON shapes.
//!
//! # Usage
//!
//! ```no_run
//! use pallas_configs::shelley;
//!
//! let config = shelley::from_file(std::path::Path::new("genesis.json"))?;
//!
//! if let Some(staking) = config.staking {
//!     if let Some(pools) = staking.pools {
//!         for (pool_id, pool) in pools {
//!             println!("pool {pool_id} has pledge {}", pool.pledge);
//!         }
//!     }
//! }
//! # Ok::<_, std::io::Error>(())
//! ```
//!
//! # Overview
//!
//! - [`byron`], [`shelley`], [`alonzo`], [`conway`] — one module per era,
//!   each exposing a `GenesisFile` (or equivalent) struct and a `from_file`
//!   helper.
//! - [`cost_models`] — typed views over Plutus cost-model tables, shared
//!   across eras.

/// Alonzo-era genesis parameters (cost models, prices, max collateral).
pub mod alonzo;
/// Byron-era genesis configuration.
pub mod byron;
/// Conway-era genesis parameters (governance, committees, hard-fork init).
pub mod conway;
/// Built-in Plutus V1/V2/V3 cost-model snapshots.
pub mod cost_models;
/// Shelley-era genesis configuration (network start, system start, k, …).
pub mod shelley;
