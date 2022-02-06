# Pallas

Rust-native building blocks for the Cardano blockchain ecosystem.

## Introduction

Pallas is an expanding collection of modules that re-implements common
Cardano logic in native Rust. This crate doesn't provide any particular
application, it is meant to be used as a base layer to facilitate the
development of higher-level use-cases, such as explorers, wallets, etc (who
knows, maybe even a full node in the far away future).

## Unboxing

The repository is organized as a Cargo workspace. Each _Pallas_ "building block" lives in its own crate. The root `pallas` crate serves as an all-in-one dependency that re-exports all of the other modules in an hierarchically organized fashion, using Cargo `features` to tailor the setup for each use-case.

As already explained, _Pallas_ aims at being an expanding set of components. The following tables describe the currently available crates, as well as the planned ones.

### Ouroboros Network

| Crates                                                   | Description                                                                      |
| ---------------------------------------------------------| -------------------------------------------------------------------------------- |
| [pallas-machines](/pallas-machines)                      | A framework for implementing state machines for Ouroboros network mini-protocols |
| [pallas-multiplexer](/pallas-multiplexer)                | Multithreaded Ouroboros multiplexer implementation using mpsc channels           |
| [pallas-handshake](/pallas-machines/src/handshake)       | Implementation of the Ouroboros network handshake mini-protocol                  |
| [pallas-blockfetch](/pallas-machines/src/blockfetch)     | Implementation of the Ouroboros network blockfetch mini-protocol                 |
| [pallas-chainsync](/pallas-machines/src/chainsync)       | Implementation of the Ouroboros network chainsync mini-protocol                  |
| [pallas-localstate](/pallas-machines/src/localstate)     | Implementation of the Ouroboros network local state query mini-protocol          |
| [pallas-txsubmission](/pallas-machines/src/txsubmission) | Implementation of the Ouroboros network txsubmission mini-protocol               |

### Ouroboros Consensus

| Crates            | Description                                               |
| ----------------- | --------------------------------------------------------- |
| pallas-leadership | Implementation of the slot leadership selection algorithm |
| pallas-selection  | Implementation of the consensus chain-selection algorithm |

### Ouroboros Ledger

| Crates                          | Description                                                             |
| ------------------------------- | ----------------------------------------------------------------------- |
| [pallas-alonzo](/pallas-alonzo) | Ledger primitives and cbor codec for the Alonzo era                     |
| pallas-ticking                  | Time passage implementation for consensus algorithm                     |
| pallas-applying                 | Logic for validating and applying new blocks and txs to the chain state |
| pallas-forecasting              | Ledger forecasting algorithm to be used by the consensus layer          |

## Etymology

> Pallas: (Greek mythology) goddess of wisdom and useful arts and prudent warfare;
