//! Cryptographic Hash for Cardano
//!
//! we expose two helper objects:
//!
//! * [`Hasher`] to help streaming objects or bytes into a hasher and computing
//!   a hash without allocating extra memory due to the required **CBOR**
//!   encoding for everything by the cardano protocol
//! * [`struct@Hash`] a conveniently strongly typed byte array
//!
//! The algorithm exposed here is `Blake2b`. We currently support two
//! digest size for the algorithm: 224 bits and 256 bits. They are the
//! only two required to use with the Cardano protocol
//!
//! # Example
//!
//! ```
//! use pallas_crypto::hash::Hasher;
//!
//! let mut hasher = Hasher::<224>::new();
//! hasher.input(b"my key");
//!
//! let digest = hasher.finalize();
//! # assert_eq!(
//! #   "276fd18711931e2c0e21430192dbeac0e458093cd9d1fcd7210f64b3",
//! #   hex::encode(digest)
//! # );
//! ```

#[allow(clippy::module_inception)]
mod hash;
mod hasher;
mod serde;

pub use self::{hash::Hash, hasher::Hasher};
