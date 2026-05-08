# Pallas Network 2

A new take on the Ouroboros networking stack that prioritises P2P
operation over the *client / server* shape used by `pallas-network`. The
public API is split between an `Interface` (where IO happens) and a
`Behavior` (the business logic), reconciled by a `Manager` — a layout
inspired by libp2p's swarm.

Once this crate is thoroughly tested and adopted by downstream clients,
`network2` is intended to replace the original `pallas-network`.

## Usage

A typical setup pairs a transport (an `Interface` impl, e.g. the
TCP-backed `TcpInterface`) with a protocol (a `Behavior` impl, e.g. the
node-to-node `InitiatorBehavior`) and drives both through a `Manager`.
The `Manager` polls the interface for IO events, hands them to the
behavior, and pushes any commands the behavior emits back at the
interface — leaving you to consume the behavior's external events. The
example below is illustrative (error handling and the `await` runtime
are elided):

```rust,ignore
use pallas_network2::{
    behavior::{AnyMessage, InitiatorBehavior, InitiatorCommand, InitiatorEvent},
    interface::TcpInterface,
    Manager, PeerId,
};

let interface = TcpInterface::<AnyMessage>::new();
let behavior  = InitiatorBehavior::default();

let mut manager = Manager::new(interface, behavior);

manager.execute(InitiatorCommand::IncludePeer(
    "relays-new.cardano-mainnet.iohk.io:3001".parse::<PeerId>().unwrap(),
));

while let Some(event) = manager.poll_next().await {
    match event {
        InitiatorEvent::PeerInitialized(pid, _) => println!("up: {pid}"),
        InitiatorEvent::BlockHeaderReceived(pid, header, _) => {
            println!("hdr from {pid}: {} bytes", header.cbor.len());
        }
        _ => {}
    }
}
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
