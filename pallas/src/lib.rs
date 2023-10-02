//! Rust-native building blocks for the Cardano blockchain ecosystem
//!
//! Pallas is an expanding collection of modules that re-implements common
//! Cardano logic in native Rust. This crate doesn't provide any particular
//! application, it is meant to be used as a base layer to facilitate the
//! development of higher-level use-cases, such as explorers, wallets, etc (who
//! knows, maybe even a full node in the far away future).

#![warn(missing_docs)]
#![warn(missing_doc_code_examples)]

#[doc(inline)]
pub use pallas_network as network;

pub mod ledger {
    //! Ledger primitives and cbor codecs for different Cardano eras

    #[doc(inline)]
    pub use pallas_primitives as primitives;

    #[doc(inline)]
    pub use pallas_traverse as traverse;

    #[doc(inline)]
    pub use pallas_addresses as addresses;
}

#[doc(inline)]
pub use pallas_crypto as crypto;

#[doc(inline)]
pub use pallas_codec as codec;

pub mod interop {
    //! Interoperability with other protocols, formats & systems

    #[doc(inline)]
    pub use pallas_utxorpc as utxorpc;
}

pub use pallas_applying as applying;
