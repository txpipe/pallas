use std::{
    collections::HashMap,
    pin::{Pin, pin},
};

use futures::{FutureExt, future::FusedFuture, select, stream::FuturesUnordered};

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

pub enum InterfaceError {}

pub type Channel = u16;
pub type Payload = Vec<u8>;

/// Describes a message that can be sent over the network
pub trait Message: Send + 'static {
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
pub trait Behavior: Sized {
    /// The event type that is raised by the behavior
    type Event;

    /// The command type that can be handled by the behavior
    type Command: Command;

    /// The state type of a peer in the network
    type PeerState: Default;

    /// The message type that is sent over the network
    type Message: Message;

    async fn poll_next(&mut self) -> Option<InterfaceCommand<Self::Message>>;

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
            cmd = behavior.poll_next().fuse() => {
                if let Some(cmd) = cmd {
                    self.interface.execute(cmd);
                }

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

#[cfg(test)]
mod tests {
    use std::pin::Pin;

    use futures::stream::{FuturesUnordered, StreamExt};
    use pallas_network::miniprotocols::Agent;

    use super::*;

    type ChainPoint = u64;
    type BlockHeader = Vec<u8>;
    type BlockBody = Vec<u8>;
    type TxId = Vec<u8>;

    enum NetworkEvent {
        PeerInitialized(PeerId),
        BlockHeaderRequested(PeerId, ChainPoint),
        BlockHeaderReceived(PeerId, BlockHeader),
        BlockBodyReceived(PeerId, ChainPoint, BlockBody),
        BlockBodyRequested(PeerId, ChainPoint),
        TxRequested(PeerId, TxId),
    }

    enum MyCommand {
        IncludePeer(PeerId),
        IntersectOrigin(PeerId),
        RequestBlockHeader(PeerId, ChainPoint),
        RequestBlockBody(PeerId, ChainPoint),
        RequestTx(PeerId, TxId),
        SendBlockHeader(PeerId, BlockHeader),
        SendBlockBody(PeerId, ChainPoint, BlockBody),
        SendTx(PeerId, TxId),
    }

    impl super::Command for MyCommand {
        fn peer_id(&self) -> &PeerId {
            match self {
                Self::IncludePeer(x) => x,
                Self::IntersectOrigin(x) => x,
                Self::RequestBlockHeader(x, _) => x,
                Self::RequestBlockBody(x, _) => x,
                Self::RequestTx(x, _) => x,
                Self::SendBlockHeader(x, _) => x,
                Self::SendBlockBody(x, _, _) => x,
                Self::SendTx(x, _) => x,
            }
        }
    }

    #[derive(Default)]
    struct MyEmulatorRules;

    impl emulation::Rules for MyEmulatorRules {
        type Message = MyMessage;

        fn reply_to(&self, msg: Self::Message) -> emulation::ReplyAction<Self::Message> {
            match msg {
                MyMessage::Handshake(msg) => match msg {
                    pallas_network::miniprotocols::handshake::Message::Propose(version_table) => {
                        let (version, data) = version_table.values.into_iter().next().unwrap();

                        let msg = pallas_network::miniprotocols::handshake::Message::Accept(
                            version, data,
                        );

                        emulation::ReplyAction::Message(MyMessage::Handshake(msg))
                    }
                    _ => emulation::ReplyAction::Disconnect,
                },
            }
        }
    }

    #[derive(Clone)]
    enum MyMessage {
        Handshake(
            pallas_network::miniprotocols::handshake::Message<
                pallas_network::miniprotocols::handshake::n2n::VersionData,
            >,
        ),
    }

    impl Message for MyMessage {
        fn channel(&self) -> Channel {
            match self {
                MyMessage::Handshake(..) => 0,
            }
        }

        fn payload(&self) -> Payload {
            match self {
                MyMessage::Handshake(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
            }
        }
    }

    #[derive(Default)]
    struct MyPeerState {
        is_connected: bool,
        is_connecting: bool,
        should_intersect: Option<u64>,
        intersected: Option<u64>,
        handshake: pallas_network::miniprotocols::handshake::N2NClient,
    }

    struct MyBehavior {
        outbound: OutboundQueue<MyMessage>,
        desired_peers: usize,
        connected_count: usize,
        connecting_count: usize,
    }

    impl Behavior for MyBehavior {
        type Event = NetworkEvent;
        type Command = MyCommand;
        type PeerState = MyPeerState;
        type Message = MyMessage;

        async fn poll_next(&mut self) -> Option<InterfaceCommand<Self::Message>> {
            self.outbound.poll_next().await
        }

        fn apply_io(
            &mut self,
            pid: &PeerId,
            state: &mut Self::PeerState,
            event: InterfaceEvent<Self::Message>,
        ) -> Option<NetworkEvent> {
            match event {
                InterfaceEvent::Connected(pid) => {
                    println!("connected to {pid}");
                    state.is_connected = true;
                    state.is_connecting = false;
                    self.connected_count += 1;
                    self.connecting_count -= 1;

                    Some(NetworkEvent::PeerInitialized(pid.clone()))
                }
                InterfaceEvent::Disconnected(pid) => {
                    println!("disconnected from {pid}");
                    state.is_connected = false;
                    state.is_connecting = false;
                    self.connected_count -= 1;

                    None
                }
                InterfaceEvent::Recv(pid, msg) => {
                    println!("received msg from {pid}, channel {}", msg.channel());

                    match msg {
                        MyMessage::Handshake(msg) => {
                            let new_state = state.handshake.apply(&msg).unwrap();
                            state.handshake =
                                pallas_network::miniprotocols::handshake::N2NClient::new(new_state);
                            println!("new handshake state {:?}", state.handshake.state());

                            if matches!(
                                state.handshake.state(),
                                pallas_network::miniprotocols::handshake::State::Propose
                            ) {
                                let msg = pallas_network::miniprotocols::handshake::Message::Propose(
                                    pallas_network::miniprotocols::handshake::n2n::VersionTable::v11_and_above(0),
                                );

                                self.outbound.push_ready(InterfaceCommand::Send(
                                    pid.clone(),
                                    MyMessage::Handshake(msg),
                                ));
                            }
                        }
                    }

                    None
                }
                InterfaceEvent::Sent(pid, msg) => {
                    println!("sent msg to {pid}, channel {}", msg.channel());

                    match msg {
                        MyMessage::Handshake(msg) => {
                            let new_state = state.handshake.apply(&msg).unwrap();
                            state.handshake =
                                pallas_network::miniprotocols::handshake::N2NClient::new(new_state);
                        }
                    }

                    None
                }
                _ => None,
            }
        }

        fn apply_cmd(&mut self, pid: &PeerId, state: &mut Self::PeerState, cmd: Self::Command) {
            match cmd {
                MyCommand::IncludePeer(_) => {
                    println!("including peer {pid}");

                    if state.is_connected && state.is_connecting {
                        return;
                    }

                    if (self.connected_count + self.connecting_count) >= self.desired_peers {
                        return;
                    }

                    println!("requesting connection to {}", pid);
                    self.connecting_count += 1;
                    self.outbound
                        .push_ready(InterfaceCommand::Connect(pid.clone()));
                }
                MyCommand::IntersectOrigin(peer_id) => {
                    println!("requesting origin intersection for {pid}");
                    state.should_intersect = Some(0);
                }
                _ => (),
            }
        }
    }

    type MyEmulator = emulation::Emulator<MyMessage, MyEmulatorRules>;

    struct MyNode {
        network: Manager<MyEmulator, MyBehavior, MyMessage>,
    }

    impl MyNode {
        async fn tick(&mut self) {
            let event = self.network.poll_next().await;

            let Some(event) = event else {
                return;
            };

            let next_cmd = match event {
                NetworkEvent::PeerInitialized(peer_id) => {
                    println!("Peer initialized: {peer_id}");
                    Some(MyCommand::IntersectOrigin(peer_id))
                }
                NetworkEvent::BlockHeaderRequested(peer_id, _) => {
                    println!("Block header requested from {peer_id}");
                    Some(MyCommand::SendBlockHeader(peer_id, vec![]))
                }
                NetworkEvent::BlockHeaderReceived(peer_id, _) => {
                    println!("Block header received from {peer_id}");
                    None
                }
                NetworkEvent::BlockBodyReceived(peer_id, _, _) => {
                    println!("Block body received from {peer_id}");
                    None
                }
                NetworkEvent::BlockBodyRequested(peer_id, point) => {
                    println!("Block body requested from {peer_id}");
                    Some(MyCommand::SendBlockBody(peer_id, point, vec![]))
                }
                NetworkEvent::TxRequested(peer_id, _) => {
                    println!("Tx requested from {peer_id}");
                    Some(MyCommand::SendTx(peer_id, vec![]))
                }
            };

            if let Some(cmd) = next_cmd {
                self.network.enqueue(cmd);
            }
        }
    }

    #[tokio::test]
    async fn test_network() {
        let behavior = MyBehavior {
            desired_peers: 3,
            connected_count: 0,
            connecting_count: 0,
            outbound: OutboundQueue::new(),
        };

        let mut node = MyNode {
            network: Manager::new(MyEmulator::default(), behavior),
        };

        [1234, 1235, 1236, 1237, 1238]
            .into_iter()
            .map(|port| PeerId {
                host: "127.0.0.1".to_string(),
                port,
            })
            .for_each(|x| node.network.enqueue(MyCommand::IncludePeer(x)));

        for _ in 0..20 {
            node.tick().await;
        }
    }
}
