# Following the Cardano Leios testnet

This example connects a node to the public Cardano **Leios** ("Musashi Dojo")
testnet and follows its endorsement layer live: it negotiates a Leios-capable
handshake, runs ordinary Praos chain-sync, and reacts to **Endorser Block (EB)**
notifications by fetching their bodies and transactions over Pallas's
[`pallas-network2`](../../pallas-network2) stack.

It's a small, single-file program (`src/main.rs`, ~290 lines) meant to be read
top-to-bottom. By the end of this tutorial you'll understand:

- what the Leios overlay is and how it rides on top of Praos;
- how a single handshake version "turns Leios on";
- the `leios-notify` (server-push) vs `leios-fetch` (client-pull) split;
- how to drive `pallas-network2`'s `Manager` / `InitiatorBehavior` event loop.

## What is Leios?

[Leios (CIP-0164)](https://cips.cardano.org/cip/CIP-0164) is an overlay on top
of Ouroboros Praos. Alongside the normal Praos chain, block producers diffuse
**Endorser Blocks** — compact lists of *transaction references* — which are then
voted on and certified back into a ranking block. The goal is higher throughput:
transactions get endorsed in parallel with normal block production.

Pallas speaks this overlay through two node-to-node mini-protocols that ride the
**same** TCP connection as Praos, once a Leios-capable handshake version is
negotiated:

| Mini-protocol  | Direction      | What it carries                                         | Surfaced as              |
| -------------- | -------------- | ------------------------------------------------------- | ------------------------ |
| `leios-notify` | server → push  | announces/offers new EBs and their txs, diffuses votes  | `InitiatorEvent::EbNotification` |
| `leios-fetch`  | client → pull  | request an EB body, or a subset of its transactions     | `InitiatorEvent::EbFetched`      |

The interaction is a notify-then-fetch loop: the relay *tells you* what it has,
and you *pull* the pieces you want.

```
relay ──leios-notify──▶  BlockOffer(eb)        "I have EB e"
  you ──leios-fetch───▶  FetchEb(e)            "send me its body"
relay ──leios-fetch───▶  Block(body)           body = { tx_hash => size } map
  you                    (remember tx count)
relay ──leios-notify──▶  BlockTxsOffer(eb)     "I can serve e's transactions"
  you ──leios-fetch───▶  FetchEbTxs(e, bitmap) "send me these txs"
relay ──leios-fetch───▶  BlockTxs { txs }      the actual transactions
```

## Prerequisites

- A Rust toolchain matching the workspace `rust-version`.
- Network access to the public Leios relay.

> **Heads up — this is a throwaway devnet.** The Musashi Dojo testnet is
> continuously reset. If the connection is refused or sync stalls, the relay
> address, network magic, or intersection point in `src/main.rs` may simply be
> stale. Check the
> [Leios testnet getting-started guide](https://leios.cardano-scaling.org/docs/testnet/getting-started/)
> for current values.

## Running it

From the repository root:

```sh
RUST_LOG=info cargo run -p leios-testnet
```

There are no command-line arguments — everything is configured by the constants
at the top of `src/main.rs` (see [Configuration](#configuration)). For more
detail, including per-transaction fetch logging and otherwise-unhandled events,
bump the log level:

```sh
RUST_LOG=debug cargo run -p leios-testnet
```

## Reading the output

A healthy run prints something like this (abridged). Each line corresponds
directly to an event handled in `src/main.rs`:

```
INFO connecting to Leios testnet relay="leios-node.play.dev.cardano.org:3001" magic=164
INFO peer initialized pid=... version=15 leios=true
INFO intersection found pid=... point=Specific(2812236, ...) tip=...
INFO header received pid=... variant=... tip_block=...
INFO EB offered → fetching body  pid=... eb=2812240@9d8a... size=1234
INFO EB body fetched             pid=... eb=2812240@9d8a... bytes=1234 txs=42
INFO txs offered → fetching      pid=... eb=2812240@9d8a... want=42 total=42
INFO EB transactions fetched     pid=... eb=2812240@9d8a... count=42 bytes=98765
INFO votes received              pid=... count=7
```

What to look for:

- **`peer initialized ... leios=true`** — the handshake negotiated a
  Leios-capable version. If you see `leios=false` (plus a warning), the peer
  spoke a pre-Leios version and **no EBs will be diffused**.
- **`intersection found` / `header received`** — ordinary Praos chain-sync,
  running underneath the overlay.
- **`EB offered` → `EB body fetched` → `txs offered` → `EB transactions
  fetched`** — the notify-then-fetch loop in action.
- **`votes received`** — Leios votes diffused inline over `leios-notify`.

## How it works

Everything lives in the `LeiosNode` struct in `src/main.rs`.

### 1. Turning Leios on (the handshake)

This is the only thing that "enables" Leios. The default `InitiatorBehavior`
proposes only a mainnet v13 handshake, which does **not** carry the overlay. The
example swaps in a version table that proposes v11–v15 with the testnet's
network magic, so the peer can negotiate v15 (`LEIOS_MIN_VERSION`, the Dijkstra
era) and bring up `leios-notify` / `leios-fetch`:

```rust
let behavior = InitiatorBehavior {
    handshake: HandshakeBehavior::new(HandshakeConfig {
        supported_version: VersionTable::v11_and_above_with_query(
            LEIOS_TESTNET_MAGIC, // 164
            false,
        ),
    }),
    ..Default::default() // chain-sync, block-fetch, keepalive stay at defaults
};
```

### 2. The event loop

`tick()` uses `tokio::select!` to multiplex two sources: a 3-second housekeeping
timer (which drives `InitiatorCommand::Housekeeping`, keeping the protocols
pumping) and the network's event stream (`Manager::poll_next`). Every event is
dispatched to `handle_event`:

```rust
select! {
    _ = self.housekeeping_interval.tick() => {
        self.network.execute(InitiatorCommand::Housekeeping);
    }
    evt = self.network.poll_next() => {
        if let Some(evt) = evt { self.handle_event(evt); }
    }
}
```

### 3. Praos chain-sync, underneath

`IntersectionFound`, `BlockHeaderReceived`, and `RollbackReceived` are ordinary
Praos events. The example just logs them and asks for more with
`InitiatorCommand::ContinueSync`. Note that chain-sync is started near the tip
(via an intersection point) rather than from origin — the Leios overlay diffuses
EBs over the same connection regardless of where you are in chain-sync.

### 4. Reacting to notifications (`handle_notification`)

This is the heart of the example. Each `leios-notify` notification triggers the
appropriate `leios-fetch` pull:

- **`BlockOffer(eb_id, size)`** → `InitiatorCommand::FetchEb(pid, eb_id)`. We
  pull the body first, because the body tells us how many transactions the EB
  has — which we need to request them correctly later.
- **`BlockTxsOffer(eb_id)`** → `InitiatorCommand::FetchEbTxs(pid, eb_id, bitmap)`,
  but **only** if we already fetched that EB's body (so we know its tx count).
  The transactions are selected with a bitmap, capped at `MAX_TXS_PER_FETCH`
  (64) per request:

  ```rust
  let want = n.min(MAX_TXS_PER_FETCH);
  self.network.execute(InitiatorCommand::FetchEbTxs(
      pid, eb_id, leiosfetch::Bitmaps::all(want),
  ));
  ```

  Two subtleties worth understanding:
  - We only request transactions a peer has **offered**. Requesting txs a peer
    hasn't offered makes the prototype relay reset the connection.
  - Each request is bounded to one 64-tx bitmap window. Asking for a whole large
    EB at once can exceed the relay's per-response limits; a real client would
    page across windows.

- **`BlockAnnouncement(raw)`** and **`Votes(votes)`** are simply logged.

### 5. Sizing the transaction request

How do we know how many transactions an EB has? The EB body is a CBOR
`{ tx_hash => size }` map, so the number of entries *is* the transaction count.
`eb_tx_count` decodes the map header (handling both definite- and
indefinite-length encodings) and the result is stashed in the `eb_tx_counts`
map, keyed by `EbId`, so it's ready when the peer later offers the transactions.

## Configuration

All knobs are constants at the top of `src/main.rs`. Because the testnet resets
periodically, expect to update these from time to time:

| Constant               | Default                                | When to change                                              |
| ---------------------- | -------------------------------------- | ----------------------------------------------------------- |
| `LEIOS_RELAY`          | `leios-node.play.dev.cardano.org:3001` | Connection refused / relay moved — check the docs.          |
| `LEIOS_TESTNET_MAGIC`  | `164`                                  | If the testnet's network magic changes.                     |
| `INTERSECT_SLOT` / `INTERSECT_HASH` | slot `2812236`, hash `9d8a43aa…` | Sync stalls / intersection not found — use a current point. |
| `MAX_TXS_PER_FETCH`    | `64`                                   | Tune how many txs to pull per `leios-fetch` request.        |

## Troubleshooting

- **Connection refused** — the relay address is likely stale (the devnet was
  reset). Get the current address from the
  [getting-started guide](https://leios.cardano-scaling.org/docs/testnet/getting-started/)
  and update `LEIOS_RELAY`.
- **Sync stalls / "intersection not found"** — the chain was reset past your
  `INTERSECT_SLOT`/`INTERSECT_HASH`. Replace them with a current point.
- **`peer negotiated a pre-Leios version`** — you connected, but the peer only
  speaks pre-v15; no EBs will be diffused. Confirm you're hitting a Leios relay.

## Going further

- The mini-protocol types live in
  [`pallas_network2::protocol::{leiosnotify, leiosfetch}`](../../pallas-network2/src/protocol);
  the initiator event loop is in
  [`pallas_network2::behavior::initiator`](../../pallas-network2/src/behavior).
- Ideas to extend this example: persist fetched EBs and transactions, page
  across **all** transaction windows of a large EB (not just the first 64),
  decode and follow the diffused votes, or connect to multiple relays at once.
- Background reading:
  [CIP-0164](https://cips.cardano.org/cip/CIP-0164) and the
  [Leios testnet docs](https://leios.cardano-scaling.org/docs/testnet/getting-started/).
