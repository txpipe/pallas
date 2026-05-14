//! Interoperability with implementation-specific artifacts of the Haskell
//! Cardano node.
//!
//! Today the main job is reading the node's immutable on-disk chunks (the
//! `immutable/` directory of a synced node), so a Rust process can iterate
//! the chain without re-syncing. A small [`display`] module also covers the
//! textual representations the upstream node uses for human-facing output.
//!
//! # Usage
//!
//! ```no_run
//! use std::path::Path;
//! use pallas_hardano::storage::immutable;
//!
//! for block in immutable::read_blocks(Path::new("/var/cardano/data/immutable"))? {
//!     let _bytes = block?;
//!     // hand `_bytes` to pallas-traverse / pallas-primitives for typed access
//! }
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
//!
//! # Overview
//!
//! - [`storage::immutable`] — readers over the node's chunk / primary /
//!   secondary index files. Top-level entry points:
//!   [`storage::immutable::read_blocks`],
//!   [`storage::immutable::read_blocks_from_point`],
//!   [`storage::immutable::get_tip`].
//! - [`display`] — pretty-printing helpers for the structures above.
//!
//! # Usage as part of `pallas`
//!
//! When depending on the umbrella [`pallas`] crate (with the `hardano`
//! feature), this crate is re-exported as
//! `pallas::interop::hardano::storage`.
//!
//! [`pallas`]: https://crates.io/crates/pallas

/// Pretty-printing helpers for the structures exposed by [`storage`].
pub mod display;
/// Readers over the Haskell Cardano node's on-disk chain database.
pub mod storage;
