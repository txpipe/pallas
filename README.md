<div align="center">
    <img src="https://raw.githubusercontent.com/txpipe/pallas/master/assets/logo-dark.svg?sanitize=true#gh-dark-mode-only" alt="Pallas Logo" width="500">
    <img src="https://raw.githubusercontent.com/txpipe/pallas/master/assets/logo-light.svg?sanitize=true#gh-light-mode-only" alt="Pallas Logo" width="500">
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

As already explained, _Pallas_ aims at being an expanding set of components. The following tables describe the currently available crates, as well as the planned ones.

### Ouroboros Network

| Crates                                        | Description                                                             |
| --------------------------------------------- | ----------------------------------------------------------------------- |
| [pallas-network](/pallas-network)             | Network stack providing a multiplexer and mini-protocol implementations |

### Ouroboros Consensus

| Crates            | Description                                               |
| ----------------- | --------------------------------------------------------- |
| pallas-leadership | Implementation of the slot leadership selection algorithm |
| pallas-selection  | Implementation of the consensus chain-selection algorithm |

### Cardano Ledger

| Crates                                  | Description                                                             |
| --------------------------------------- | ----------------------------------------------------------------------- |
| [pallas-primitives](/pallas-primitives) | Ledger primitives and cbor codec for the different Cardano eras         |
| [pallas-traverse](/pallas-traverse)     | Utilities to traverse over multi-era block data                         |
| [pallas-addresses](/pallas-addresses)   | Encode / decode Cardano addresses of any type                           |
| pallas-ticking                          | Time passage implementation for consensus algorithm                     |
| pallas-applying                         | Logic for validating and applying new blocks and txs to the chain state |
| pallas-forecasting                      | Ledger forecasting algorithm to be used by the consensus layer          |

### Shared

| Crates                          | Description                                        |
| ------------------------------- | -------------------------------------------------- |
| [pallas-crypto](/pallas-crypto) | Shared Cryptographic primitives                    |
| [pallas-codec](/pallas-codec)   | Shared CBOR encoding / decoding using minicbor lib |
| [pallas-math](/pallas-math)     | Shared mathematics functions                       |

## Etymology

> Pallas: (Greek mythology) goddess of wisdom and useful arts and prudent warfare;
