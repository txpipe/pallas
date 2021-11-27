# Pallas

Rust-native building blocks for the Cardano blockchain ecosystem.

## Introduction

Pallas is an expanding collection of modules that re-implements common
Cardano logic in native Rust. This crate doesn't provide any particular
application, it is meant to be used as a base layer to facilitate the
development of higher-level use-cases, such as explorers, wallets, etc (who
knows, maybe even a full node in the far away future).

## Unboxing
| Crates              | Description                                                                      |
|---------------------|----------------------------------------------------------------------------------|
| [pallas-machines](/pallas-machines)     | A framework for implementing state machines for Ouroboros network mini-protocols |
| [pallas-multiplexer](/pallas-multiplexer)  | A multithreaded Ouroboros multiplexer implementation using mpsc channels         |
| [pallas-handshake](/pallas-handshake)    | An implementation of the Ouroboros network handshake mini-protocol               |
| [pallas-blockfetch](/pallas-blockfetch)   | An implementation of the Ouroboros network blockfetch mini-protocol              |
| [pallas-chainsync](/pallas-chainsync)    | An implementation of the Ouroboros network chainsync mini-protocol               |
| [pallas-txsubmission](/pallas-txsubmission) | An implementation of the Ouroboros network txsubmission mini-protocol            |
| [pallas-alonzo](/pallas-alonzo)       | Ledger primitives and cbor codec for the Alonzo era                              |
| pallas              | An all-in-one crate that re-exports the other ones in an ordered fashion         |

## Etymology

> Pallas: (Greek mythology) goddess of wisdom and useful arts and prudent warfare;
