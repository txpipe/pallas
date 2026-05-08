# Pallas Network 2

A new take on the Ouroboros networking stack that prioritises P2P
operation over the *client / server* shape used by `pallas-network`. The
public API is split between an `Interface` (where IO happens) and a
`Behavior` (the business logic), reconciled by a `Manager` — a layout
inspired by libp2p's swarm.

Once this crate is thoroughly tested and adopted by downstream clients,
`network2` is intended to replace the original `pallas-network`.

## Usage

The control flow is the same regardless of which `Interface` /
`Behavior` you plug in. Sketch (substitute your own values for
`interface`, `behavior`, and any commands):

```text
use pallas_network2::Manager;

let mut manager = Manager::new(interface, behavior);

while let Some(event) = manager.poll_next().await {
    // event has type `<B as Behavior>::Event`
}

manager.execute(command);
```

## Overview

- `Manager` — drives a paired `Interface` + `Behavior`. `poll_next` advances
  IO and the behavior; `execute` forwards an external command to the
  behavior.
- `Interface` trait — the IO side. Receives `InterfaceCommand` (Connect /
  Send / Disconnect) and yields `InterfaceEvent` (Connected / Disconnected /
  Sent / Recv / Error / Idle).
- `Behavior` trait — the protocol logic. Defines its own `Event`,
  `Command`, `PeerState`, and `Message`, and emits `BehaviorOutput`s.
- `Message` trait — describes a mini-protocol message (channel id +
  payload encoding).
- `OutboundQueue` — convenience queue of pending `BehaviorOutput`s ready
  to be polled by the manager.
- `PeerId`, `Channel`, `Payload`, `MAX_SEGMENT_PAYLOAD_LENGTH` — the
  primitive vocabulary.

### Modules

- `bearer` — low-level transport for reading and writing multiplexed segments.
- `interface` — `Interface` implementations for TCP connections.
- `behavior` — opinionated `Behavior` implementations for Cardano stacks.
- `protocol` — the Ouroboros mini-protocol definitions (handshake,
  chainsync, blockfetch, …).

## Feature flags

- `emulation` — enables the `emulation` module, an in-memory test harness
  for exercising behaviors without real network IO.
