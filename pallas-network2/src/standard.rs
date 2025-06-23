//! Opinionated standard behavior for Cardano networks

use pallas_network::miniprotocols::{
    Agent, Point,
    blockfetch::{self, Body},
    chainsync, handshake, keepalive, peersharing, txsubmission,
};

use crate::{Behavior, Command, InterfaceEvent, Message, PeerId};

impl Command for () {
    fn peer_id(&self) -> &PeerId {
        unreachable!()
    }
}

#[derive(Debug, Clone)]
pub enum AnyMessage {
    Handshake(handshake::Message<handshake::n2n::VersionData>),
    KeepAlive(keepalive::Message),
    ChainSync(chainsync::Message<chainsync::HeaderContent>),
    PeerSharing(peersharing::Message),
    BlockFetch(blockfetch::Message),
    TxSubmission(txsubmission::Message<txsubmission::EraTxId, txsubmission::EraTxBody>),
}

impl Message for AnyMessage {
    fn channel(&self) -> u16 {
        match self {
            AnyMessage::Handshake(_) => pallas_network::miniprotocols::PROTOCOL_N2N_HANDSHAKE,
            AnyMessage::KeepAlive(_) => pallas_network::miniprotocols::PROTOCOL_N2N_KEEP_ALIVE,
            AnyMessage::ChainSync(_) => pallas_network::miniprotocols::PROTOCOL_N2N_CHAIN_SYNC,
            AnyMessage::PeerSharing(_) => pallas_network::miniprotocols::PROTOCOL_N2N_PEER_SHARING,
            AnyMessage::BlockFetch(_) => pallas_network::miniprotocols::PROTOCOL_N2N_BLOCK_FETCH,
            AnyMessage::TxSubmission(_) => {
                pallas_network::miniprotocols::PROTOCOL_N2N_TX_SUBMISSION
            }
        }
    }

    fn payload(&self) -> Vec<u8> {
        match self {
            AnyMessage::Handshake(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
            AnyMessage::KeepAlive(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
            AnyMessage::ChainSync(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
            AnyMessage::PeerSharing(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
            AnyMessage::BlockFetch(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
            AnyMessage::TxSubmission(msg) => pallas_codec::minicbor::to_vec(msg).unwrap(),
        }
    }
}

pub struct HandshakeBehavior {
    supported_versions: handshake::n2n::VersionTable,
}

impl Default for HandshakeBehavior {
    fn default() -> Self {
        Self {
            supported_versions: handshake::n2n::VersionTable::v11_and_above(0),
        }
    }
}

impl Behavior for HandshakeBehavior {
    type Event = ();
    type Command = ();
    type PeerState = handshake::Client<handshake::n2n::VersionData>;
    type Message = AnyMessage;

    fn schedule_io(
        &mut self,
        pid: &PeerId,
        peer: &mut Self::PeerState,
    ) -> Option<crate::InterfaceCommand<Self::Message>> {
        match peer.state() {
            handshake::State::Propose => {
                let msg = handshake::Message::Propose(self.supported_versions.clone());

                Some(crate::InterfaceCommand::Send(
                    pid.clone(),
                    AnyMessage::Handshake(msg),
                ))
            }
            _ => None,
        }
    }

    fn apply_io(
        &mut self,
        pid: &PeerId,
        state: &mut Self::PeerState,
        event: crate::InterfaceEvent<Self::Message>,
    ) -> Option<Self::Event> {
        let InterfaceEvent::Recv(_, AnyMessage::Handshake(msg)) = event else {
            return None;
        };

        let new_state = state.apply(&msg).unwrap();
        *state = handshake::Client::new(new_state);

        None
    }

    fn apply_cmd(&mut self, _pid: &PeerId, _state: &mut Self::PeerState, _cmd: Self::Command) {
        unreachable!()
    }
}

pub struct ChainSyncBehavior;
pub struct PeerSharingBehavior;
pub struct BlockFetchBehavior;
pub struct TxSubmissionBehavior;

pub type LastSeen = chrono::DateTime<chrono::Utc>;

#[derive(PartialEq)]
pub enum PeerPriority {
    Cold,
    Warm,
    Hot,
    Banned,
}

impl Default for PeerPriority {
    fn default() -> Self {
        Self::Cold
    }
}

#[derive(PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected(LastSeen),
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}

#[derive(Default)]
pub struct InitiatorState {
    connection: ConnectionState,
    priority: PeerPriority,
    handshake: handshake::Client<handshake::n2n::VersionData>,
    handshake_inflight: bool,
}

impl InitiatorState {
    fn requires_connection(&self) -> bool {
        if self.connection == ConnectionState::Disconnected {
            if self.priority == PeerPriority::Warm || self.priority == PeerPriority::Hot {
                return true;
            }
        }

        return false;
    }

    fn requires_handshake(&self) -> bool {
        !self.handshake.is_done() && !self.handshake_inflight
    }
}

pub enum InitiatorCommand {
    IncludePeer(PeerId),
    IntersectChain(PeerId, Point),
    RequestNextHeader(PeerId, Point),
    RequestBlockBody(PeerId, Point),
    SendTx(PeerId, txsubmission::EraTxId, txsubmission::EraTxBody),
}

impl Command for InitiatorCommand {
    fn peer_id(&self) -> &PeerId {
        match self {
            Self::IncludePeer(pid) => pid,
            Self::IntersectChain(pid, _) => pid,
            Self::RequestNextHeader(pid, _) => pid,
            Self::RequestBlockBody(pid, _) => pid,
            Self::SendTx(pid, _, _) => pid,
        }
    }
}

pub enum InitiatorEvent {
    PeerInitialized(PeerId),
    BlockHeaderReceived(PeerId, chainsync::HeaderContent),
    BlockBodyReceived(PeerId, Point, Body),
    TxRequested(PeerId, txsubmission::EraTxId),
}

pub struct DiscoveryConfig {
    max_peers: usize,
    max_warm_peers: usize,
    max_hot_peers: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            max_peers: 10,
            max_warm_peers: 5,
            max_hot_peers: 3,
        }
    }
}

#[derive(Default)]
pub struct DiscoveryStats {
    peers: usize,
    warm_peers: usize,
    hot_peers: usize,
}

#[derive(Default)]
pub struct InitiatorBehavior {
    config: DiscoveryConfig,
    stats: DiscoveryStats,
    handshake: HandshakeBehavior,
}

impl Behavior for InitiatorBehavior {
    type Event = InitiatorEvent;
    type Command = InitiatorCommand;
    type PeerState = InitiatorState;
    type Message = AnyMessage;

    fn schedule_io(
        &mut self,
        pid: &PeerId,
        state: &mut Self::PeerState,
    ) -> Option<crate::InterfaceCommand<Self::Message>> {
        if state.requires_connection() {
            return Some(crate::InterfaceCommand::Connect(pid.clone()));
        }

        if state.requires_handshake() {
            state.handshake_inflight = true;
            return self.handshake.schedule_io(pid, &mut state.handshake);
        }

        None
    }

    fn apply_io(
        &mut self,
        pid: &PeerId,
        state: &mut Self::PeerState,
        event: crate::InterfaceEvent<Self::Message>,
    ) -> Option<Self::Event> {
        match &event {
            crate::InterfaceEvent::Recv(_, msg) => match msg {
                AnyMessage::Handshake(_) => {
                    self.handshake.apply_io(&pid, &mut state.handshake, event);
                    None
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn apply_cmd(&mut self, _pid: &PeerId, state: &mut Self::PeerState, cmd: Self::Command) {
        match cmd {
            InitiatorCommand::IncludePeer(_) => {
                if self.stats.peers >= self.config.max_peers {
                    return;
                }

                state.priority = PeerPriority::Warm;
                self.stats.peers += 1;
            }
            _ => (),
        }
    }
}

pub struct ResponderBehavior;
pub struct ResponderState;

pub enum ResponderEvent {}

pub enum ResponderCommand {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Manager, emulation};

    #[derive(Default)]
    struct MyEmulatorRules;

    impl emulation::Rules for MyEmulatorRules {
        type Message = AnyMessage;

        fn reply_to(&self, msg: Self::Message) -> emulation::ReplyAction<Self::Message> {
            match msg {
                AnyMessage::Handshake(msg) => match msg {
                    pallas_network::miniprotocols::handshake::Message::Propose(version_table) => {
                        let (version, data) = version_table.values.into_iter().next().unwrap();

                        let msg = pallas_network::miniprotocols::handshake::Message::Accept(
                            version, data,
                        );

                        emulation::ReplyAction::Message(AnyMessage::Handshake(msg))
                    }
                    _ => emulation::ReplyAction::Disconnect,
                },
                AnyMessage::KeepAlive(msg) => {
                    let keepalive::Message::KeepAlive(token) = msg else {
                        return emulation::ReplyAction::Disconnect;
                    };

                    let msg = keepalive::Message::ResponseKeepAlive(token);

                    emulation::ReplyAction::Message(AnyMessage::KeepAlive(msg))
                }
                _ => todo!(),
            }
        }
    }

    type MyEmulator = emulation::Emulator<AnyMessage, MyEmulatorRules>;

    struct MyNode {
        network: Manager<MyEmulator, InitiatorBehavior, AnyMessage>,
    }

    impl MyNode {
        async fn tick(&mut self) {
            let event = self.network.poll_next().await;

            let Some(event) = event else {
                return;
            };

            let next_cmd = match event {
                InitiatorEvent::PeerInitialized(peer_id) => {
                    println!("Peer initialized: {peer_id}");
                    Some(InitiatorCommand::IntersectChain(peer_id, Point::Origin))
                }

                InitiatorEvent::BlockHeaderReceived(peer_id, _) => {
                    println!("Block header received from {peer_id}");
                    None
                }
                InitiatorEvent::BlockBodyReceived(peer_id, _, _) => {
                    println!("Block body received from {peer_id}");
                    None
                }
                InitiatorEvent::TxRequested(peer_id, _) => {
                    println!("Tx requested from {peer_id}");
                    Some(InitiatorCommand::SendTx(
                        peer_id,
                        txsubmission::EraTxId(0, vec![]),
                        txsubmission::EraTxBody(0, vec![]),
                    ))
                }
            };

            if let Some(cmd) = next_cmd {
                self.network.enqueue(cmd);
            }
        }
    }

    #[tokio::test]
    async fn test_network() {
        let mut node = MyNode {
            network: Manager::new(MyEmulator::default(), InitiatorBehavior::default()),
        };

        [1234, 1235, 1236, 1237, 1238]
            .into_iter()
            .map(|port| PeerId {
                host: "127.0.0.1".to_string(),
                port,
            })
            .for_each(|x| node.network.enqueue(InitiatorCommand::IncludePeer(x)));

        for _ in 0..20 {
            node.tick().await;
        }
    }
}
