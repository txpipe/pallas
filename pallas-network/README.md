# Pallas Network

Implementation of the Ouroboros networking stack — a multiplexer plus
state-machines for each Cardano mini-protocol (handshake, chainsync,
blockfetch, txsubmission, local-state-query, …) — exposed through
ergonomic per-role facades. Async, tokio-backed.

This is the original, single-connection / client-server-shaped stack. A
peer-to-peer rewrite is in progress in `pallas-network2`; once that is
mature it is intended to replace this crate.

## Usage

```rust
use pallas_network::facades::PeerClient;
use pallas_network::miniprotocols::MAINNET_MAGIC;

let mut peer = PeerClient::connect(
    "relays-new.cardano-mainnet.iohk.io:3001",
    MAINNET_MAGIC,
).await?;

let _chainsync  = peer.chainsync();   // &mut chainsync::N2NClient
let _blockfetch = peer.blockfetch();  // &mut blockfetch::Client
```

## Overview

- `facades` — opinionated client / server bundles per role:
  `PeerClient` / `PeerServer` (N2N), `NodeClient` / `NodeServer` (N2C),
  `DmqClient` / `DmqServer`. Each owns a multiplexer and exposes the
  relevant mini-protocols through accessor methods.
- `multiplexer` — the segment-level transport: `RunningPlexer`, `Bearer`,
  `VersionTable`, `VersionNumber`.
- `miniprotocols` — every Ouroboros mini-protocol: `handshake`, `chainsync`,
  `blockfetch`, `txsubmission`, `localstate`, `localtxsubmission`,
  `txmonitor`, `keepalive`, `peersharing`, `localmsgnotification`,
  `localmsgsubmission`. Network-magic constants (`MAINNET_MAGIC`,
  `TESTNET_MAGIC`, `PREVIEW_MAGIC`, `PREPROD_MAGIC`, `SANCHONET_MAGIC`) are
  re-exported from this module.
