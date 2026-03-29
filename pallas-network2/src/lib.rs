//! P2P Network stack compatible with the Ouroboros protocol
use std::{fmt::Debug, pin::Pin};

use futures::{
    Stream, StreamExt, select,
    stream::{FusedStream, FuturesUnordered},
};

#[cfg(feature = "emulation")]
pub mod emulation;

/// Low-level transport layer for reading and writing multiplexed segments.
pub mod bearer;
/// Opinionated behavior implementations for Cardano network stacks.
pub mod behavior;
/// Network interface implementations for TCP connections.
pub mod interface;
/// Ouroboros mini-protocol definitions (handshake, chainsync, blockfetch, etc.).
pub mod protocol;

/// A unique identifier for a peer in the network
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct PeerId {
    /// The hostname or IP address of the peer.
    pub host: String,
    /// The TCP port of the peer.
    pub port: u16,
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

impl std::str::FromStr for PeerId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (host, port) = s.split_once(':').ok_or("invalid peer id")?;
        Ok(PeerId {
            host: host.to_string(),
            port: port.parse().unwrap(),
        })
    }
}

/// An error that occurred within the network interface.
#[derive(Debug)]
pub enum InterfaceError {
    // TODO: add more specific errors
    /// A generic error with a human-readable description.
    Other(String),
}

/// A multiplexer channel identifier for a mini-protocol.
pub type Channel = u16;

/// Raw bytes of a mini-protocol message payload.
pub type Payload = Vec<u8>;

/// Protocol value that defines max segment length
pub const MAX_SEGMENT_PAYLOAD_LENGTH: usize = 65535;

/// Describes a message that can be sent over the network
pub trait Message: Send + 'static + std::fmt::Debug + Sized + Clone + Debug {
    /// Returns the channel identifier for this message's mini-protocol.
    fn channel(&self) -> Channel;
    /// Encodes this message into its raw payload bytes.
    fn payload(&self) -> Payload;

    /// Try to decode a message from a payload.
    ///
    /// This method should use a best-effort approach to decode a message from
    /// the payload. Implementors need to take into account that payload might
    /// be partial, in this case should return none and wait for a new call with
    /// more data.
    ///
    /// Whatever payload is successfully consumed during the parsing, should be
    /// drained from the variable, leaving the remaining data available for a
    /// next call which will be used in the next attempt.
    fn from_payload(channel: Channel, payload: &mut Payload) -> Option<Self>;

    /// Converts this message into its channel and raw payload bytes.
    fn into_payload(self) -> (Channel, Payload);

    /// Converts this message into its channel and a list of payload chunks,
    /// each respecting [`MAX_SEGMENT_PAYLOAD_LENGTH`].
    fn into_chunks(self) -> (Channel, Vec<Payload>) {
        let (channel, payload) = self.into_payload();

        let chunks = payload
            .chunks(MAX_SEGMENT_PAYLOAD_LENGTH)
            .map(Vec::from)
            .collect();

        (channel, chunks)
    }
}

/// A low-level command to interact with the network interface
#[derive(Debug)]
pub enum InterfaceCommand<M: Message> {
    /// Initiate a connection to the given peer.
    Connect(PeerId),
    /// Send a message to an already-connected peer.
    Send(PeerId, M),
    /// Disconnect from the given peer.
    Disconnect(PeerId),
}

/// A low-level event from the network interface
#[derive(Debug)]
pub enum InterfaceEvent<M: Message> {
    /// A connection to the peer was successfully established.
    Connected(PeerId),
    /// The peer has been disconnected.
    Disconnected(PeerId),
    /// A message was successfully sent to the peer.
    Sent(PeerId, M),
    /// One or more messages were received from the peer.
    Recv(PeerId, Vec<M>),
    /// An error occurred on the connection to the peer.
    Error(PeerId, InterfaceError),
    /// No pending IO activity; useful for triggering housekeeping.
    Idle,
}

/// Output produced by a [`Behavior`], either a command for the interface or an
/// event for the external consumer.
#[derive(Debug)]
pub enum BehaviorOutput<B: Behavior> {
    /// A command to be dispatched to the network interface.
    InterfaceCommand(InterfaceCommand<B::Message>),
    /// An event to be surfaced to the caller.
    ExternalEvent(B::Event),
}

impl<B: Behavior> From<InterfaceCommand<B::Message>> for BehaviorOutput<B> {
    fn from(cmd: InterfaceCommand<B::Message>) -> Self {
        BehaviorOutput::InterfaceCommand(cmd)
    }
}

/// An abstraction over the network interface where IO happens
#[trait_variant::make]
pub trait Interface<M: Message>: Unpin + FusedStream + Stream<Item = InterfaceEvent<M>> {
    /// Dispatch a command to the interface (connect, send, or disconnect).
    fn dispatch(&mut self, cmd: InterfaceCommand<M>);
}

/// Describes the behavior (business logic) of a network stack
#[trait_variant::make]
pub trait Behavior:
    Sized + Unpin + FusedStream + Stream<Item = BehaviorOutput<Self>> + Send + 'static
{
    /// The event type that is raised by the behavior
    type Event: Debug + Send + 'static;

    /// The command type that can be handled by the behavior
    type Command;

    /// The state type of a peer in the network
    type PeerState: Default;

    /// The message type that is sent over the network
    type Message: Message + Debug + Send + 'static;

    /// Apply an IO event to the behavior
    ///
    /// This is the hook where a behavior can apply an event coming from the
    /// network interface.
    ///
    /// The behavior is responsible for updating the state of the peer to
    /// reflect the what has been received from the network interface.
    fn handle_io(&mut self, event: InterfaceEvent<Self::Message>);

    /// Execute an external command on the behavior.
    fn execute(&mut self, cmd: Self::Command);
}

/// Manager to reconcile state between a network interface and a behavior
pub struct Manager<I, B, M>
where
    M: Message,
    I: Interface<M>,
    B: Behavior<Message = M>,
{
    interface: I,
    behavior: B,
}

impl<I, B, M> Manager<I, B, M>
where
    M: Message,
    I: Interface<M>,
    B: Behavior<Message = M>,
{
    /// Creates a new manager from an interface and a behavior.
    pub fn new(interface: I, behavior: B) -> Self {
        Self {
            interface,
            behavior,
        }
    }

    /// Polls the interface and behavior, returning the next external event if
    /// available. Interface commands produced by the behavior are dispatched
    /// automatically.
    pub async fn poll_next(&mut self) -> Option<B::Event> {
        let Self {
            behavior,
            interface,
            ..
        } = self;

        select! {
            output = behavior.select_next_some() => {
                match output {
                    BehaviorOutput::InterfaceCommand(cmd) => {
                        self.interface.dispatch(cmd);
                        None
                    }
                    BehaviorOutput::ExternalEvent(event) => {
                        Some(event)
                    }
                }
            },
            event = interface.select_next_some() => {
                self.behavior.handle_io(event);
                None
            }
        }
    }

    /// Forwards an external command to the underlying behavior.
    pub fn execute(&mut self, cmd: B::Command) {
        self.behavior.execute(cmd);
    }
}

/// A queue of pending [`BehaviorOutput`] items ready to be polled by the
/// manager.
pub struct OutboundQueue<B: Behavior> {
    futures: FuturesUnordered<Pin<Box<dyn Future<Output = BehaviorOutput<B>> + Send + Unpin>>>,
}

impl<B: Behavior> OutboundQueue<B> {
    /// Creates an empty outbound queue.
    pub fn new() -> Self {
        Self {
            futures: FuturesUnordered::new(),
        }
    }

    /// Enqueues an output that is immediately ready.
    pub fn push_ready(&mut self, output: impl Into<BehaviorOutput<B>>) {
        self.futures
            .push(Box::pin(futures::future::ready(output.into())));
    }

    /// Polls for the next available output.
    pub async fn poll_next(&mut self) -> Option<BehaviorOutput<B>> {
        futures::stream::StreamExt::next(&mut self.futures).await
    }
}

impl<B: Behavior> Default for OutboundQueue<B> {
    fn default() -> Self {
        Self::new()
    }
}
