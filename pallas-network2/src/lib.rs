use std::{collections::HashMap, pin::Pin};

use futures::{
    FutureExt, Stream, StreamExt, select,
    stream::{FusedStream, FuturesUnordered},
};

#[cfg(feature = "emulation")]
pub mod emulation;

pub mod standard;

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
    Idle,
}

impl<M: Message> InterfaceEvent<M> {
    fn peer_id(&self) -> Option<&PeerId> {
        match self {
            InterfaceEvent::Connected(x) => Some(x),
            InterfaceEvent::Sent(x, ..) => Some(x),
            InterfaceEvent::Recv(x, ..) => Some(x),
            InterfaceEvent::Disconnected(x) => Some(x),
            InterfaceEvent::Idle => None,
        }
    }
}

/// An abstraction over the network interface where IO happens
#[trait_variant::make]
pub trait Interface<M: Message> {
    fn execute(&mut self, cmd: InterfaceCommand<M>) -> Result<(), InterfaceError>;
    async fn poll_next(&mut self) -> InterfaceEvent<M>;
}

/// Describes a command that can be handled by a behavior
pub trait Command {
    fn peer_id(&self) -> &PeerId;
}

/// Describes the behavior (business logic) of a network stack
#[trait_variant::make]
pub trait Behavior:
    Sized + Unpin + FusedStream + Stream<Item = InterfaceCommand<Self::Message>>
{
    /// The event type that is raised by the behavior
    type Event;

    /// The command type that can be handled by the behavior
    type Command: Command;

    /// The state type of a peer in the network
    type PeerState: Default;

    /// The message type that is sent over the network
    type Message: Message;

    /// Apply an IO event to the behavior
    ///
    /// This is the hook where a behavior can apply an event coming from the
    /// network interface.
    ///
    /// The behavior is responsible for updating the state of the peer to
    /// reflect the what has been received from the network interface.
    fn apply_io(
        &mut self,
        pid: &PeerId,
        state: &mut Self::PeerState,
        event: InterfaceEvent<Self::Message>,
    ) -> Option<Self::Event>;

    fn apply_cmd(&mut self, pid: &PeerId, state: &mut Self::PeerState, cmd: Self::Command);
}

/// The state of a peer in the network
pub struct Peer<B: Behavior> {
    id: PeerId,
    state: B::PeerState,
}

impl<B: Behavior> Peer<B> {
    fn new(id: PeerId) -> Self {
        Self {
            id,
            state: B::PeerState::default(),
        }
    }

    fn get_state_mut(&mut self) -> &mut B::PeerState {
        &mut self.state
    }
}

/// Manager to reconcile state between a network interface and a behavior
pub struct Manager<I, B, M>
where
    M: Message,
    I: Interface<M>,
    B: Behavior<Message = M>,
{
    peers: HashMap<PeerId, Peer<B>>,
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
            peers: HashMap::new(),
            backlog: Vec::new(),
            interface,
            behavior,
        }
    }

    async fn inbound_io(&mut self, event: InterfaceEvent<M>) -> Option<B::Event> {
        let pid = event.peer_id().cloned()?;

        let peer = self
            .peers
            .entry(pid.clone())
            .or_insert_with(|| Peer::new(pid.clone()));

        let state = peer.get_state_mut();
        self.behavior.apply_io(&pid, state, event)
    }

    fn apply_cmds(&mut self) {
        for cmd in self.backlog.drain(..) {
            let pid = cmd.peer_id().clone();

            let peer = self
                .peers
                .entry(pid.clone())
                .or_insert_with(|| Peer::new(pid.clone()));

            let state = peer.get_state_mut();
            self.behavior.apply_cmd(&pid, state, cmd);
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
            cmd = behavior.select_next_some() => {
                // TODO: define interface error handling
                self.interface.execute(cmd).unwrap();


                None
            },
            event = interface.poll_next().fuse() => {
                self.inbound_io(event).await
            }
        }
    }

    pub fn enqueue(&mut self, cmd: B::Command) {
        self.backlog.push(cmd);
    }
}

pub struct OutboundQueue<M: Message> {
    futures: FuturesUnordered<Pin<Box<dyn Future<Output = InterfaceCommand<M>> + Send + Unpin>>>,
}

impl<M: Message + Send + 'static> OutboundQueue<M> {
    pub fn new() -> Self {
        Self {
            futures: FuturesUnordered::new(),
        }
    }

    pub fn push_ready(&mut self, command: InterfaceCommand<M>) {
        self.futures.push(Box::pin(futures::future::ready(command)));
    }

    pub async fn poll_next(&mut self) -> Option<InterfaceCommand<M>> {
        futures::stream::StreamExt::next(&mut self.futures).await
    }
}

impl<M: Message + Send + 'static> Default for OutboundQueue<M> {
    fn default() -> Self {
        Self::new()
    }
}
