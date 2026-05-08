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
        <img alt="GitHub Workflow Status" src="https://img.shields.io/github/actions/workflow/status/txpipe/pallas/validate.yml" />
    <hr/>
</div>

## Introduction

Pallas is an expanding collection of modules that re-implements common
Ouroboros / Cardano logic in native Rust. This crate doesn't provide any particular
application, it is meant to be used as a base layer to facilitate the
development of higher-level use-cases, such as explorers, wallets, etc (who
knows, maybe even a full node in a far away future).

## Getting Started

Add the umbrella crate to your project:

```bash
cargo add pallas
```

Or pick a single building block:

```bash
cargo add pallas-network pallas-traverse
```

The umbrella `pallas` crate re-exports all modules behind Cargo features (see
[Features](#features) below). End-to-end usage patterns live in the
[`examples/`](/examples) directory — chain crawler, wallet key derivation, P2P
initiator/responder, and node-to-node / node-to-client mini-protocols.

## Unboxing

The repository is organized as a Cargo workspace. Each _Pallas_ "building block" lives in its own crate. The root `pallas` crate serves as an all-in-one dependency that re-exports all of the other modules in an hierarchically organized fashion, using Cargo `features` to tailor the setup for each use-case.

### Core

| Crates                          | Description                                                          |
| ------------------------------- | -------------------------------------------------------------------- |
| [pallas-codec](/pallas-codec)   | Shared CBOR encoding / decoding using minicbor lib                   |
| [pallas-crypto](/pallas-crypto) | Shared Cryptographic primitives                                      |
| [pallas-math](/pallas-math)     | Shared mathematics functions                                         |
| [pallas-bech32](/pallas-bech32) | Bech32 conventions for Cardano (CIP-5 prefixes, CIP-14 fingerprints) |

### Network

`pallas-network2` is a P2P-focused rewrite intended to eventually replace `pallas-network`. New projects should evaluate both.

| Crates                              | Description                                                           |
| ----------------------------------- | --------------------------------------------------------------------- |
| [pallas-network](/pallas-network)   | Network stack providing multiplexer and mini-protocol implementations |
| [pallas-network2](/pallas-network2) | P2P-first rewrite of the Ouroboros networking stack                   |

### Ledger

| Crates                                  | Description                                                          |
| --------------------------------------- | -------------------------------------------------------------------- |
| [pallas-primitives](/pallas-primitives) | Ledger primitives and cbor codec for the different Cardano eras      |
| [pallas-traverse](/pallas-traverse)     | Utilities to traverse over multi-era block data                      |
| [pallas-addresses](/pallas-addresses)   | Encode / decode Cardano addresses of any type                        |
| [pallas-configs](/pallas-configs)       | Genesis config structs (Byron / Shelley / Alonzo / Conway)           |
| [pallas-validate](/pallas-validate)     | Phase-1 and optional phase-2 (Plutus) transaction validation         |

### Tx Builder

| Crates                                | Description                                |
| ------------------------------------- | ------------------------------------------ |
| [pallas-txbuilder](/pallas-txbuilder) | Ergonomic transaction builder              |

## Interop

| Crates                            | Description                                                                         |
| --------------------------------- | ----------------------------------------------------------------------------------- |
| [pallas-hardano](/pallas-hardano) | Interoperability with implementation-specific artifacts of the Haskell Cardano node |
| [pallas-utxorpc](/pallas-utxorpc) | Interoperability with the [UTxO RPC](https://utxorpc.org) specification             |

## Features

The umbrella `pallas` crate exposes the following Cargo features:

| Feature    | Enables                                                       |
| ---------- | ------------------------------------------------------------- |
| `hardano`  | Haskell-node interop via `pallas-hardano`                     |
| `phase2`   | Plutus script validation in `pallas-validate`                 |
| `network2` | Opt in to the new P2P networking stack (`pallas-network2`)    |
| `relaxed`  | Relaxed validation modes across primitives and crypto         |
| `unstable` | Aggregates feature gates that are not yet stable              |

## Etymology

> Pallas: (Greek mythology) goddess of wisdom and useful arts and prudent warfare;
