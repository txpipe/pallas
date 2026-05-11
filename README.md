<div align="center">
    <picture>
        <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/txpipe/pallas/main/assets/logo-dark.svg">
        <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/txpipe/pallas/main/assets/logo-light.svg">
        <img src="https://raw.githubusercontent.com/txpipe/pallas/main/assets/logo-light.svg" alt="Pallas Logo" width="500">
    </picture>
    <hr />
        <h3 align="center" style="border-bottom: none">Rust-native building blocks for the Cardano blockchain ecosystem</h3>
        <img alt="GitHub" src="https://img.shields.io/github/license/txpipe/pallas" />
        <img alt="Crates.io" src="https://img.shields.io/crates/v/pallas" />
        <img alt="GitHub Workflow Status" src="https://img.shields.io/github/actions/workflow/status/txpipe/pallas/ci.yml" />
    <hr/>
</div>

## Introduction

Pallas is an expanding collection of modules that re-implements common
Ouroboros / Cardano logic in native Rust. This crate doesn't provide any particular
application, it is meant to be used as a base layer to facilitate the
development of higher-level use-cases, such as explorers, wallets, etc (who
knows, maybe even a full node in a far away future).

## Getting Started

For most use-cases, depend on the umbrella `pallas` crate — it re-exports every
building block under a single, organized module tree:

```bash
cargo add pallas
```

The hierarchy mirrors the project's domains (network, ledger, crypto, etc.) so
you can pull in a single dependency and reach for what you need:

```text
pallas
├── network          — Ouroboros networking stack
├── network2         — P2P-first networking stack (feature `network2`)
├── ledger
│   ├── primitives   — multi-era CBOR primitives
│   ├── traverse     — multi-era traversal helpers
│   ├── addresses    — Cardano address codec
│   └── validate     — phase-1 / phase-2 transaction validation
├── crypto           — cryptographic primitives
├── codec            — CBOR codec (minicbor)
├── interop
│   ├── utxorpc      — UTxO RPC interop
│   └── hardano      — Haskell-node interop (feature `hardano`)
└── txbuilder        — ergonomic transaction builder
```

### Feature Flags

The umbrella crate exposes the following Cargo features:

| Feature    | Enables                                                     |
| ---------- | ----------------------------------------------------------- |
| `hardano`  | Haskell-node interop (`pallas::interop::hardano`)           |
| `phase2`   | Plutus script validation in `pallas::ledger::validate`      |
| `network2` | Opt in to the new P2P networking stack (`pallas::network2`) |
| `relaxed`  | Relaxed validation modes across primitives and crypto       |
| `unstable` | Aggregates feature gates that are not yet stable            |

## Unboxing

If you'd rather depend on a subset, every building block is published as its
own crate on crates.io. Pick only what you need:

```bash
cargo add pallas-network pallas-traverse
```

The crates are grouped below by domain.

### Core

Foundational primitives with no Cardano-specific semantics. Every higher layer in the workspace depends on them.

| Crates                          | Description                                                          |
| ------------------------------- | -------------------------------------------------------------------- |
| [pallas-codec](/pallas-codec)   | Shared CBOR encoding / decoding using minicbor lib |
| [pallas-crypto](/pallas-crypto) | Shared Cryptographic primitives                    |
| [pallas-math](/pallas-math)     | Shared mathematics functions                       |

### Network

Wire-level implementation of the Ouroboros mini-protocols used to communicate with other Cardano nodes.

| Crates                              | Description                                                           |
| ----------------------------------- | --------------------------------------------------------------------- |
| [pallas-network](/pallas-network)   | Network stack providing multiplexer and mini-protocol implementations |
| [pallas-network2](/pallas-network2) | P2P-first rewrite of the Ouroboros networking stack                   |

### Ledger

Cardano's on-chain data model: how transactions, blocks, addresses, and ledger state are represented, traversed, and validated across eras.

| Crates                                  | Description                                                          |
| --------------------------------------- | -------------------------------------------------------------------- |
| [pallas-primitives](/pallas-primitives) | Ledger primitives and cbor codec for the different Cardano eras      |
| [pallas-traverse](/pallas-traverse)     | Utilities to traverse over multi-era block data                      |
| [pallas-addresses](/pallas-addresses)   | Encode / decode Cardano addresses of any type                        |
| [pallas-validate](/pallas-validate)     | Phase-1 and optional phase-2 (Plutus) transaction validation         |

### Interop

Adapters for systems built outside Pallas — file formats, RPC schemas, etc.

| Crates                            | Description                                                                         |
| --------------------------------- | ----------------------------------------------------------------------------------- |
| [pallas-hardano](/pallas-hardano) | Interoperability with implementation-specific artifacts of the Haskell Cardano node |
| [pallas-configs](/pallas-configs) | Genesis config structs matching the Haskell node (Byron / Shelley / Alonzo / Conway) |
| [pallas-utxorpc](/pallas-utxorpc) | Interoperability with the [UTxO RPC](https://utxorpc.org) specification             |

### Utils

Optional, self-contained conveniences. Each crate stands alone; pull one in only when you need that specific affordance.

| Crates                                | Description                                                          |
| ------------------------------------- | -------------------------------------------------------------------- |
| [pallas-bech32](/pallas-bech32)       | Bech32 conventions for Cardano (CIP-5 prefixes, CIP-14 fingerprints) |
| [pallas-txbuilder](/pallas-txbuilder) | Ergonomic transaction builder                                        |

## Examples

The [`examples/`](/examples) directory contains runnable demonstrations of
common integration patterns:

| Example                                          | Description                                                          |
| ------------------------------------------------ | -------------------------------------------------------------------- |
| [block-decode](/examples/block-decode)           | Decode a Byron-era block from CBOR                                   |
| [block-download](/examples/block-download)       | Download a single block from a remote node by chain point            |
| [crawler](/examples/crawler)                     | Consume the chain-sync mini-protocol with pluggable block/tx filters |
| [n2n-miniprotocols](/examples/n2n-miniprotocols) | Node-to-node mini-protocols over TCP                                 |
| [n2c-miniprotocols](/examples/n2c-miniprotocols) | Node-to-client mini-protocols over a local Unix socket               |
| [p2p-initiator](/examples/p2p-initiator)         | Initiate a P2P connection using `pallas-network2`                    |
| [p2p-responder](/examples/p2p-responder)         | Accept incoming P2P connections using `pallas-network2`              |
| [p2p-discovery](/examples/p2p-discovery)         | Peer discovery using `pallas-network2`                               |
| [wallet](/examples/wallet)                       | Wallet key generation, BIP-39 mnemonics, address derivation          |

## Minimum Supported Rust Version

Pallas's MSRV is **Rust 1.88**. CI verifies the entire workspace builds with
that toolchain on every change. The floor is set by transitive dependencies
(currently `serde_with` / `darling`); edition 2024, used by `pallas-network2`,
contributes a hard floor of 1.85.

Bumping the MSRV is treated as a breaking change: it happens only in minor
version bumps (or in `0.x` / `1.0.0-alpha.x` while we are pre-stable), is
called out in the changelog, and aims to stay roughly within the most recent
three stable Rust releases.

## Etymology

> Pallas: (Greek mythology) goddess of wisdom and useful arts and prudent warfare;

## License

Pallas is distributed under the terms of the [Apache License 2.0](LICENSE).

## Security

If you discover a security vulnerability, please follow the disclosure process described in [SECURITY.md](SECURITY.md). Do not open a public GitHub issue.
