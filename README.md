<div align="center">
    <picture>
        <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/txpipe/pallas/master/assets/logo-dark.svg">
        <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/txpipe/pallas/master/assets/logo-light.svg">
        <img src="https://raw.githubusercontent.com/txpipe/pallas/master/assets/logo-light.svg" alt="Pallas Logo" width="500">
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

## Unboxing

The repository is organized as a Cargo workspace. Each _Pallas_ "building block" lives in its own crate. The root `pallas` crate serves as an all-in-one dependency that re-exports all of the other modules in an hierarchically organized fashion, using Cargo `features` to tailor the setup for each use-case.

### Core

| Crates                          | Description                                        |
| ------------------------------- | -------------------------------------------------- |
| [pallas-codec](/pallas-codec)   | Shared CBOR encoding / decoding using minicbor lib |
| [pallas-crypto](/pallas-crypto) | Shared Cryptographic primitives                    |
| [pallas-math](/pallas-math)     | Shared mathematics functions                       |

### Network

| Crates                            | Description                                                           |
| --------------------------------- | --------------------------------------------------------------------- |
| [pallas-network](/pallas-network) | Network stack providing multiplexer and mini-protocol implementations |

### Ledger

| Crates                                  | Description                                                     |
| --------------------------------------- | --------------------------------------------------------------- |
| [pallas-primitives](/pallas-primitives) | Ledger primitives and cbor codec for the different Cardano eras |
| [pallas-traverse](/pallas-traverse)     | Utilities to traverse over multi-era block data                 |
| [pallas-addresses](/pallas-addresses)   | Encode / decode Cardano addresses of any type                   |

### Wallet

| Crates                                | Description                                |
| ------------------------------------- | ------------------------------------------ |
| [pallas-wallet](/pallas-wallet)       | Wallet utilities for secure key management |
| [pallas-txbuilder](/pallas-txbuilder) | Ergonomic transaction builder              |

## Interop

| Crates                            | Description                                                                         |
| --------------------------------- | ----------------------------------------------------------------------------------- |
| [pallas-hardano](/pallas-hardano) | Interoperability with implementation-specific artifacts of the Haskell Cardano node |
| [pallas-utxorpc](/pallas-utxorpc) | Interoperability with the [UTxO RPC](https://utxorpc.org) specification             |

## Etymology

> Pallas: (Greek mythology) goddess of wisdom and useful arts and prudent warfare;
