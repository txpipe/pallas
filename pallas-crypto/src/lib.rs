//! Cryptographic primitives required to participate in the Cardano protocol.
//!
//! Blake2b hashing, Ed25519 signing (regular and BIP32-extended), VRF, KES
//! forward-secure signatures, and the nonce evolution used by the Ouroboros
//! leader-selection schedule. Algorithm choices follow the ones made by the
//! Cardano protocol.
//!
//! # Usage
//!
//! ```
//! use pallas_crypto::hash::Hasher;
//!
//! let mut h = Hasher::<256>::new();
//! h.input(b"hello");
//! let digest = h.finalize();
//! println!("blake2b-256 = {}", digest);
//! ```
//!
//! # Overview
//!
//! - [`hash`] — `Hash<N>` and `Hasher<N>` over Blake2b. The const generic is
//!   in bits, so `Hasher::<224>` and `Hasher::<256>` cover the common
//!   Cardano digest sizes.
//! - [`key::ed25519`] — regular and extended Ed25519 key pairs, signing and
//!   verification.
//! - `kes` — KES (Key Evolving Signature) primitives used by block
//!   producers (feature `kes`).
//! - [`nonce`] — epoch / chain-nonce evolution helpers.
//! - [`memsec`] — secure-memory utilities used to wipe key material.
//!
//! # Status
//!
//! - [x] Blake2b 256
//! - [x] Blake2b 224
//! - [x] Ed25519 asymmetric key pair and EdDSA
//! - [x] Ed25519 Extended asymmetric key pair
//! - [ ] BIP32-Ed25519 key derivation
//! - [ ] BIP39 mnemonics
//! - [x] VRF
//! - [x] KES
//! - [ ] SECP256k1
//! - [x] Nonce calculations
//!
//! # Feature flags
//!
//! - `kes` — pulls in the `kes` module and its KES (Key Evolving
//!   Signature) primitives.
//! - `relaxed` — relax some validation checks; useful when round-tripping
//!   non-canonical historical artifacts.
//!
//! # Usage as part of `pallas`
//!
//! When depending on the umbrella [`pallas`] crate, this crate is re-exported
//! as `pallas::crypto`.
//!
//! [`pallas`]: https://crates.io/crates/pallas

extern crate core;

/// Fixed-size byte hashes used throughout Cardano (Blake2b-224, Blake2b-256, …).
pub mod hash;
/// KES (Key Evolving Signature) primitives used by block producers.
pub mod kes;
/// Ed25519 keys and signatures used by wallets, witnesses, and operational certificates.
pub mod key;
/// Secure-memory utilities used to wipe key material.
pub mod memsec;
/// Nonce derivation utilities used by the Ouroboros leader-selection schedule.
pub mod nonce;
