#![warn(missing_docs, rust_2018_idioms)]
//! A key evolving signatures implementation based on
//! "Composition and Efficiency Tradeoffs for Forward-Secure Digital Signatures"
//! by Tal Malkin, Daniele Micciancio and Sara Miner
//! <https://eprint.iacr.org/2001/034>
//!
//! Specfically we do the binary sum composition directly as in the paper, and
//! then use that in a nested\/recursive fashion to construct up to a 7-level
//! deep binary tree version.
//!
//! We provide two different implementations in this crate, to provide compatibility
//! with Cardano's different eras. The first, `SumKes`, is a trivial construction,
//! while the second, `SumCompactKes`, is a version with a more compact signature.
//!
//! Consider the following Merkle tree:
//!
//! ```ascii
//!       (A)
//!      /   \
//!   (B)     (C)
//!   / \     / \
//! (D) (E) (F) (G)
//!      ^
//!  0   1   2   3
//! ```
//!
//! The caret points at leaf node E, indicating that the current period is 1.
//! The signatures for leaf nodes D through G all contain their respective
//! DSIGN keys.
//!
//! In the naive `SumKes` signatures the signature for branch node B holds
//! the signature for node E, and the VerKeys for nodes D and E. The signature
//! for branch node A (the root node), the signature for node B and the
//! VerKeys for nodes B and C. In other words, the number of individual hashes
//! to be stored equals the depth of the Merkle tree.
//!
//! Instead, the more efficient `SumCompactKes` gets rid of some redundant data
//! of the signature. In particular, the signature for branch node B only holds
//! the signature for node E, and the VerKey for node D. It can reconstruct its
//! own VerKey from these two. The signature for branch node A (the root node),
//! then, only contains the VerKey for node C, and the signature for node B. In
//! other words, the number of individual hashes to be stored equals the depth
//! of the Merkle tree.

pub mod common;
pub mod errors;
mod single_kes;
pub mod summed_kes;
mod summed_kes_tests;
pub mod traits;

pub use common::PublicKey;
