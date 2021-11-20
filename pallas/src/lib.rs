//! Rust-native building blocks for the Cardano blockchain ecosystem
//!
//! Pallas is an expanding collection of modules that re-implements common
//! Cardano logic in native Rust. This crate doesn't provide any particular
//! application, it is meant to be used as a base layer to facilitate the
//! development of higher-level use-cases, such as explorers, wallets, etc (who
//! knows, maybe even a full node in the far away future).

#![warn(missing_docs)]
#![warn(missing_doc_code_examples)]

pub mod ouroboros;
