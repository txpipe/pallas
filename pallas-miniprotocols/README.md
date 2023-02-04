# Pallas Mini-protocols

This crate provides an implementation of the different Ouroboros mini-protocols as defined in the [The Shelley Networking Protocol](https://hydra.iohk.io/build/1070091/download/1/network.pdf#chapter.3) specs.

## Architectural Decisions

The following architectural decisions were made for this particular Rust implementation:

- The mini-protocols will remain agnostic of the concrete ledger implementation. For example, the block-fetch implementation is generic over the particular block data structure.
- The codec implemenation of the messages defined by the Ouroboros specs belongs to this crate, but any ledger-specific structure is out-of-scope.
- The state-machine execution will remain agnostic of the concrete mini-protocol specification.

## Development Status

| mini-protocol       | initiator | responder |
| ------------------- | --------- | --------- |
| block-fetch         | done      | planned   |
| chain-sync          | done      | planned   |
| handshake           | done      | planned   |
| local-state         | done      | planned   |
| tx-submission       | planned   | minimal   |
| local tx monitor    | done      | planned   |
| local-tx-submission | ongoing   | planned   |

## Implementation Details

An Ouroboros mini-protocol is defined as a state-machine. This library provides the primitive artifacts to describe the different states and messages of each particular state-machine.

A local agent, either initiator or responder, interacts with a remote agent by exchanging messages and keeping its own version of the state.

By implementing the following trait, a struct can participate as an agent in an Ouroboros communication:

```rust
pub trait Agent: Sized {
    type Message;

    fn is_done(&self) -> bool;
    fn has_agency(&self) -> bool;
    fn send_next(self, tx: &impl MachineOutput) -> Transition<Self>;
    fn receive_next(self, msg: Self::Message) -> Transition<Self>;
}
```

- The associate type `Message` is an enum with the particular variants of each particular miniprotocol
- The `has_agency` function describes if the agent has agency for the current state.
- The `is_done` function describes if the agent considers that all tasks have been done.
- The `send_next` function instructs the agent to send the next message in the sequence (will be called only if it has agency).
- The `receive_next` function instructs the agent to process the following received message.

The `send_next` and the `receive_next` methods will transition the state-machine from one state to the following. This transition happens without mutating any value, the idea is that each step in the process transition the agent struct into a new struct of the same type describing the new state. This approach allows us to implement the execution of the state machine as a pure function.

To tigger the execution of an agent, the library provides the following entry-point:

```rust
run_agent<T>(agent: T, channel: &mut Channel)
```

Where `T` is the type of the concrete agent to execute and the `Channel` is the Ouroboros multiplexer channel already connected to the remote party.

## Execution Example

The following example shows how to execute a Handshake client against a remote relay node.

```rust
// setup a TCP bearer against a relay node
let bearer = TcpStream::connect("relays-new.cardano-mainnet.iohk.io:3001").unwrap();
bearer.set_nodelay(true).unwrap();
bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

// create a new multiplexer, specifying which mini-protocol IDs we want to sue
let mut muxer = Multiplexer::setup(bearer, &[0]).unwrap();

// get a handle for the handhsake mini-protocol handle
let mut channel = muxer.use_channel(0);

// create a handshake client agent with an initial state 
let agent = handshake::Client::initial(VersionTable::v4_and_above(MAINNET_MAGIC));

// run the agent, which internally executes all the transitions
// until it is done.
let agent = run_agent(agent, &mut channel).unwrap();

// print the final state of the agent
println!("{agent:?}");
```