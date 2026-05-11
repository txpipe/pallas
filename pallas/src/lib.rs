//! Rust-native building blocks for the Cardano blockchain ecosystem.
//!
//! Pallas is a collection of modules that re-implements core Ouroboros /
//! Cardano logic in native Rust. It does not provide any single application;
//! it is a base layer for higher-level use-cases — explorers, indexers,
//! wallets, transaction builders, validators, and (eventually) full nodes.
//!
//! # Crate layout
//!
//! The umbrella `pallas` crate re-exports every building block under a single
//! module tree, organised by domain:
//!
//! ```text
//! pallas
//! ├── network         — Ouroboros networking stack
//! ├── network2        — P2P-first networking stack (feature `network2`)
//! ├── ledger
//! │   ├── primitives  — multi-era CBOR primitives
//! │   ├── traverse    — multi-era traversal helpers
//! │   ├── addresses   — Cardano address codec
//! │   └── validate    — phase-1 / phase-2 transaction validation
//! ├── crypto          — cryptographic primitives
//! ├── codec           — CBOR codec (minicbor)
//! ├── interop
//! │   ├── utxorpc     — UTxO RPC interop
//! │   └── hardano     — Haskell-node interop (feature `hardano`)
//! └── txbuilder       — ergonomic transaction builder
//! ```
//!
//! Each module is a thin re-export of a standalone `pallas-*` crate published
//! on crates.io ([`pallas-network`], [`pallas-primitives`], …). If you only
//! need a subset, depend on the individual crates directly.
//!
//! # Examples
//!
//! Runnable demonstrations of common integration patterns live in the
//! [`examples/`] directory of the repository:
//!
//! | Example               | Description                                                          |
//! | --------------------- | -------------------------------------------------------------------- |
//! | [`block-decode`]      | Decode a Byron-era block from CBOR                                   |
//! | [`block-download`]    | Download a single block from a remote node by chain point            |
//! | [`crawler`]           | Consume the chain-sync mini-protocol with pluggable block/tx filters |
//! | [`n2n-miniprotocols`] | Node-to-node mini-protocols over TCP                                 |
//! | [`n2c-miniprotocols`] | Node-to-client mini-protocols over a local Unix socket               |
//! | [`p2p-initiator`]     | Initiate a P2P connection using `pallas-network2`                    |
//! | [`p2p-responder`]     | Accept incoming P2P connections using `pallas-network2`              |
//! | [`p2p-discovery`]     | Peer discovery using `pallas-network2`                               |
//! | [`wallet`]            | Wallet key generation, BIP-39 mnemonics, address derivation          |
//!
//! [`block-decode`]: https://github.com/txpipe/pallas/tree/main/examples/block-decode
//! [`block-download`]: https://github.com/txpipe/pallas/tree/main/examples/block-download
//! [`crawler`]: https://github.com/txpipe/pallas/tree/main/examples/crawler
//! [`n2n-miniprotocols`]: https://github.com/txpipe/pallas/tree/main/examples/n2n-miniprotocols
//! [`n2c-miniprotocols`]: https://github.com/txpipe/pallas/tree/main/examples/n2c-miniprotocols
//! [`p2p-initiator`]: https://github.com/txpipe/pallas/tree/main/examples/p2p-initiator
//! [`p2p-responder`]: https://github.com/txpipe/pallas/tree/main/examples/p2p-responder
//! [`p2p-discovery`]: https://github.com/txpipe/pallas/tree/main/examples/p2p-discovery
//! [`wallet`]: https://github.com/txpipe/pallas/tree/main/examples/wallet
//!
//! # Feature flags
//!
//! | Feature    | Enables                                                       |
//! | ---------- | ------------------------------------------------------------- |
//! | `hardano`  | Haskell-node interop (`pallas::interop::hardano`)             |
//! | `phase2`   | Plutus script validation in [`ledger::validate`]              |
//! | `network2` | The P2P-first networking stack (`pallas::network2`)           |
//! | `relaxed`  | Relaxed validation modes across primitives and crypto         |
//! | `unstable` | Aggregates feature gates that are not yet stable              |
//!
//! Features are additive: enabling one never removes APIs exposed by another.
//! [docs.rs](https://docs.rs/pallas) builds with `all-features`.
//!
//! # Minimum Supported Rust Version
//!
//! Pallas's MSRV is **Rust 1.88**. Bumping it is treated as a breaking change
//! and is called out in the changelog.
//!
//! # License
//!
//! Distributed under the terms of the [Apache License 2.0][license]. To report
//! a security issue, follow the disclosure process in [SECURITY.md][security]
//! rather than opening a public issue.
//!
//! [`pallas-network`]: https://crates.io/crates/pallas-network
//! [`pallas-primitives`]: https://crates.io/crates/pallas-primitives
//! [`examples/`]: https://github.com/txpipe/pallas/tree/main/examples
//! [license]: https://github.com/txpipe/pallas/blob/main/LICENSE
//! [security]: https://github.com/txpipe/pallas/blob/main/SECURITY.md

#![warn(missing_docs)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/txpipe/pallas/main/assets/logo-light.svg"
)]

pub mod network {
    //! Wire-level Ouroboros networking stack.
    //!
    //! Implements the multiplexer and the node-to-node / node-to-client
    //! mini-protocols (chain-sync, block-fetch, tx-submission, local-state
    //! query, handshake, keep-alive, …) used to communicate with other
    //! Cardano nodes.
    //!
    //! Re-export of the [`pallas-network`] crate.
    //!
    //! [`pallas-network`]: https://crates.io/crates/pallas-network
    pub use pallas_network::*;
}

#[cfg(feature = "network2")]
pub mod network2 {
    //! P2P-first rewrite of the Ouroboros networking stack.
    //!
    //! Adds peer discovery and inbound connection handling on top of the
    //! mini-protocols, suitable for participating in the public relay mesh
    //! rather than only connecting out to a known node.
    //!
    //! Requires the `network2` feature. Re-export of the [`pallas-network2`]
    //! crate.
    //!
    //! [`pallas-network2`]: https://crates.io/crates/pallas-network2
    pub use pallas_network2::*;
}

pub mod ledger {
    //! Cardano's on-chain data model.
    //!
    //! How blocks, transactions, addresses, and ledger state are represented,
    //! traversed, and validated across every era from Byron to Conway.

    pub mod primitives {
        //! Ledger primitives and CBOR codec for every Cardano era.
        //!
        //! One module per era (`byron`, `alonzo`, `babbage`, `conway`, …)
        //! exposing the canonical types and their `minicbor`
        //! encode/decode implementations.
        //!
        //! Re-export of the [`pallas-primitives`] crate.
        //!
        //! [`pallas-primitives`]: https://crates.io/crates/pallas-primitives
        pub use pallas_primitives::*;
    }

    pub mod traverse {
        //! Multi-era traversal of block and transaction data.
        //!
        //! A unified API for walking over blocks, headers, transactions,
        //! inputs, outputs, assets, certificates, and metadata without
        //! branching on era at every call site.
        //!
        //! Re-export of the [`pallas-traverse`] crate.
        //!
        //! [`pallas-traverse`]: https://crates.io/crates/pallas-traverse
        pub use pallas_traverse::*;
    }

    pub mod addresses {
        //! Encode and decode Cardano addresses of any type.
        //!
        //! Covers Byron, Shelley payment, and stake addresses, with bech32,
        //! hex, and raw-byte representations.
        //!
        //! Re-export of the [`pallas-addresses`] crate.
        //!
        //! [`pallas-addresses`]: https://crates.io/crates/pallas-addresses
        pub use pallas_addresses::*;
    }

    pub mod validate {
        //! Apply Cardano ledger rules to transactions and blocks.
        //!
        //! Phase-1 covers structural and policy checks (fees, witnesses,
        //! validity intervals, …). Phase-2 evaluates Plutus scripts and
        //! reports execution units; it is gated on the `phase2` feature.
        //!
        //! Re-export of the [`pallas-validate`] crate.
        //!
        //! [`pallas-validate`]: https://crates.io/crates/pallas-validate
        pub use pallas_validate::*;
    }
}

pub mod crypto {
    //! Cryptographic primitives used across Cardano.
    //!
    //! Blake2b hashes, Ed25519 keys and signatures, KES (Key Evolving
    //! Signatures), VRF, and the nonce derivation used by Ouroboros leader
    //! selection. Algorithms target the choices made by the Cardano
    //! protocol.
    //!
    //! Re-export of the [`pallas-crypto`] crate.
    //!
    //! [`pallas-crypto`]: https://crates.io/crates/pallas-crypto
    pub use pallas_crypto::*;
}

pub mod codec {
    //! Shared CBOR codec for Cardano data structures.
    //!
    //! Built on [`minicbor`]; also re-exports the Plutus Core flat codec and
    //! the round-trip helper types (`Bytes`, `Nullable`, `Set`, …) that the
    //! rest of Pallas depends on.
    //!
    //! Re-export of the [`pallas-codec`] crate.
    //!
    //! [`pallas-codec`]: https://crates.io/crates/pallas-codec
    pub use pallas_codec::codec_by_datatype;
    pub use pallas_codec::*;
}

// TODO: re-incorporate math here once we commit to a final set of upstream
// dependencies

// pub mod math {
//     //! Cardano-specific math (rational arithmetic, fixed-point decimals).
//     #[cfg(feature = "pallas-math")]
//     pub use pallas_math::*;
// }

pub mod interop {
    //! Adapters for systems built outside Pallas.
    //!
    //! Bridges to external file formats, RPC schemas, and node artifacts so
    //! Pallas types can be consumed from — and produced for — the wider
    //! Cardano tooling ecosystem.

    pub mod utxorpc {
        //! Convert between Pallas types and the [UTxO RPC] wire format.
        //!
        //! Re-export of the [`pallas-utxorpc`] crate.
        //!
        //! [UTxO RPC]: https://utxorpc.org
        //! [`pallas-utxorpc`]: https://crates.io/crates/pallas-utxorpc
        pub use pallas_utxorpc::*;
    }

    #[cfg(feature = "hardano")]
    pub mod hardano {
        //! Interop with implementation-specific artifacts of the Haskell
        //! Cardano node.
        //!
        //! Read on-disk chain data written by the upstream node. Gated on
        //! the `hardano` feature.
        //!
        //! Re-export of the [`pallas-hardano`] crate.
        //!
        //! [`pallas-hardano`]: https://crates.io/crates/pallas-hardano
        pub use pallas_hardano::*;

        pub mod configs {
            //! Genesis configs matching the Haskell Cardano node.
            //!
            //! Strongly-typed structs for the Byron, Shelley, Alonzo, and Conway
            //! genesis files.
            //!
            //! Re-export of the [`pallas-configs`] crate.
            //!
            //! [`pallas-configs`]: https://crates.io/crates/pallas-configs
            pub use pallas_configs::*;
        }
    }
}

pub mod txbuilder {
    //! Ergonomic builder for Cardano transactions.
    //!
    //! Stage inputs, outputs, mints, certificates, and witnesses through a
    //! fluent API, then call into an era-specific builder (e.g. Conway) to
    //! produce a fully-encoded, signable transaction.
    //!
    //! Re-export of the [`pallas-txbuilder`] crate.
    //!
    //! [`pallas-txbuilder`]: https://crates.io/crates/pallas-txbuilder
    pub use pallas_txbuilder::*;
}
