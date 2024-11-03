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

    #[doc(inline)]
    // WARNING: this is deprecated, use `pallas::interop::hardano::configs` instead.
    // Since deprecation notices don't work for re-exports we don't have a way to notify users.
    pub use pallas_configs as configs;

    #[doc(inline)]
    #[cfg(feature = "pallas-applying")]
    pub use pallas_applying as rules;
}

#[doc(inline)]
pub use pallas_crypto as crypto;

#[doc(inline)]
pub use pallas_codec as codec;

#[doc(inline)]
#[cfg(feature = "pallas-math")]
pub use pallas_math as math;

pub mod interop {
    //! Interoperability with other protocols, formats & systems

    #[doc(inline)]
    pub use pallas_utxorpc as utxorpc;

    #[cfg(feature = "pallas-hardano")]
    pub mod hardano {
        //! Interoperability with the Haskell Cardano node

        #[doc(inline)]
        pub use pallas_hardano::storage;

        #[doc(inline)]
        pub use pallas_configs as configs;
    }
}

pub mod storage {
    //! Storage engines for chain-related persistence

    #[cfg(feature = "pallas-hardano")]
    #[doc(inline)]
    // WARNING: this is deprecated, use `pallas::interop::hardano::storage` instead.
    // Since deprecation notices don't work for re-exports we don't have a way to notify users.
    pub use pallas_hardano::storage as hardano;
}

#[doc(inline)]
#[cfg(feature = "pallas-applying")]
// WARNING: this is deprecated but since deprecation notices don't work for re-exports
// we don't have a way to notify users.
pub use pallas_applying as applying;

#[cfg(feature = "wallet")]
pub mod wallet {
    //! Utilities for wallet implementations

    #[doc(inline)]
    #[cfg(feature = "pallas-wallet")]
    pub use pallas_wallet as keystore;

    #[doc(inline)]
    pub use pallas_txbuilder as txbuilder;
}

#[doc(inline)]
// WARNING: this is deprecated, use `pallas::wallet::txbuilder` instead.
// Since deprecation notices don't work for re-exports we don't have a way to notify users.
pub use pallas_txbuilder as txbuilder;
