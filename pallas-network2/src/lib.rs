use std::{fmt::Debug, pin::Pin};

use futures::{
    FutureExt, Stream, StreamExt, select,
    stream::{FusedStream, FuturesUnordered},
};

#[cfg(feature = "emulation")]
pub mod emulation;

pub mod behavior;

/// A unique identifier for a peer in the network
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct PeerId {
    pub host: String,
    pub port: u16,
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

#[derive(Debug)]
pub enum InterfaceError {}

pub type Channel = u16;
pub type Payload = Vec<u8>;

/// Describes a message that can be sent over the network
pub trait Message: Send + 'static + std::fmt::Debug {
    fn channel(&self) -> Channel;
    fn payload(&self) -> Payload;
}

/// A low-level command to interact with the network interface
#[derive(Debug)]
pub enum InterfaceCommand<M: Message> {
    Connect(PeerId),
    Send(PeerId, M),
    Disconnect(PeerId),
}

/// A low-level event from the network interface
#[derive(Debug)]
pub enum InterfaceEvent<M: Message> {
    Connected(PeerId),
    Disconnected(PeerId),
    Sent(PeerId, M),
    Recv(PeerId, M),
    Error(PeerId, InterfaceError),
    Idle,
}

#[derive(Debug)]
pub enum BehaviorOutput<B: Behavior> {
    InterfaceCommand(InterfaceCommand<B::Message>),
    ExternalEvent(B::Event),
}

impl<B: Behavior> From<InterfaceCommand<B::Message>> for BehaviorOutput<B> {
    fn from(cmd: InterfaceCommand<B::Message>) -> Self {
        BehaviorOutput::InterfaceCommand(cmd)
    }
}

/// An abstraction over the network interface where IO happens
#[trait_variant::make]
pub trait Interface<M: Message> {
    fn dispatch(&mut self, cmd: InterfaceCommand<M>);
    async fn poll_next(&mut self) -> InterfaceEvent<M>;
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
    fn apply_io(&mut self, event: InterfaceEvent<Self::Message>);

    fn apply_cmd(&mut self, cmd: Self::Command);
}

/// Manager to reconcile state between a network interface and a behavior
pub struct Manager<I, B, M>
where
    M: Message,
    I: Interface<M>,
    B: Behavior<Message = M>,
{
    backlog: Vec<B::Command>,
    interface: I,
    behavior: B,
}

impl<I, B, M> Manager<I, B, M>
where
    M: Message,
    I: Interface<M>,
    B: Behavior<Message = M>,
{
    pub fn new(interface: I, behavior: B) -> Self {
        Self {
            backlog: Vec::new(),
            interface,
            behavior,
        }
    }

    pub fn behavior(&self) -> &B {
        &self.behavior
    }

    fn apply_cmds(&mut self) {
        for cmd in self.backlog.drain(..) {
            self.behavior.apply_cmd(cmd);
        }
    }

    pub async fn poll_next(&mut self) -> Option<B::Event> {
        self.apply_cmds();

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
            event = interface.poll_next().fuse() => {
                self.behavior.apply_io(event);
                None
            }
        }
    }

    pub fn enqueue(&mut self, cmd: B::Command) {
        self.backlog.push(cmd);
    }
}

pub struct OutboundQueue<B: Behavior> {
    futures: FuturesUnordered<Pin<Box<dyn Future<Output = BehaviorOutput<B>> + Send + Unpin>>>,
}

impl<B: Behavior> OutboundQueue<B> {
    pub fn new() -> Self {
        Self {
            futures: FuturesUnordered::new(),
        }
    }

    pub fn push_ready(&mut self, output: impl Into<BehaviorOutput<B>>) {
        self.futures
            .push(Box::pin(futures::future::ready(output.into())));
    }

    pub async fn poll_next(&mut self) -> Option<BehaviorOutput<B>> {
        futures::stream::StreamExt::next(&mut self.futures).await
    }
}

impl<B: Behavior> Default for OutboundQueue<B> {
    fn default() -> Self {
        Self::new()
    }
}
