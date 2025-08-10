//! Opinionated standard behavior for Cardano networks

use std::{task::Poll, time::Duration};

use chrono::DateTime;
use futures::{Stream, StreamExt, stream::FusedStream};
use pallas_network::miniprotocols::{
    Agent, Point,
    blockfetch::{self, Body},
    chainsync, handshake, keepalive, peersharing, txsubmission,
};
use tokio::time::Interval;

use crate::{Behavior, Command, InterfaceEvent, Message, OutboundQueue, PeerId};

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

impl AnyMessage {
    fn as_handshake(&self) -> Option<&handshake::Message<handshake::n2n::VersionData>> {
        match self {
            AnyMessage::Handshake(x) => Some(x),
            _ => None,
        }
    }
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
    outbound: OutboundQueue<AnyMessage>,
}

impl Default for HandshakeBehavior {
    fn default() -> Self {
        Self {
            supported_versions: handshake::n2n::VersionTable::v11_and_above(0),
            outbound: OutboundQueue::new(),
        }
    }
}

impl Stream for HandshakeBehavior {
    type Item = crate::InterfaceCommand<AnyMessage>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.outbound.futures.poll_next_unpin(cx)
    }
}

impl FusedStream for HandshakeBehavior {
    fn is_terminated(&self) -> bool {
        false
    }
}

impl Behavior for HandshakeBehavior {
    type Event = InitiatorEvent;
    type Command = ();
    type PeerState = handshake::Client<handshake::n2n::VersionData>;
    type Message = AnyMessage;

    fn apply_io(
        &mut self,
        pid: &PeerId,
        state: &mut Self::PeerState,
        event: crate::InterfaceEvent<Self::Message>,
    ) -> Option<Self::Event> {
        let new_state = match event {
            InterfaceEvent::Connected(_) => {
                let sm = handshake::State::<handshake::n2n::VersionData>::default();
                handshake::Client::new(sm)
            }
            InterfaceEvent::Sent(_, msg) => {
                let msg = msg.as_handshake().unwrap();
                let sm = state.apply(&msg).unwrap();
                handshake::Client::new(sm)
            }
            InterfaceEvent::Recv(_, msg) => {
                let msg = msg.as_handshake().unwrap();
                let sm = state.apply(&msg).unwrap();
                handshake::Client::new(sm)
            }
            _ => {
                return None;
            }
        };

        *state = new_state;

        match state.state() {
            handshake::State::Propose => {
                let msg = handshake::Message::Propose(self.supported_versions.clone());

                self.outbound.push_ready(crate::InterfaceCommand::Send(
                    pid.clone(),
                    AnyMessage::Handshake(msg),
                ));

                None
            }
            handshake::State::Done(_) => Some(InitiatorEvent::PeerInitialized(pid.clone())),
            _ => None,
        }
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
    Connected,
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

pub struct InitiatorBehavior {
    config: DiscoveryConfig,
    stats: DiscoveryStats,
    handshake: HandshakeBehavior,
    outbound: OutboundQueue<AnyMessage>,
    housekeeping: Interval,
}

impl Default for InitiatorBehavior {
    fn default() -> Self {
        Self {
            config: Default::default(),
            stats: Default::default(),
            handshake: Default::default(),
            outbound: Default::default(),
            housekeeping: tokio::time::interval(Duration::from_secs(1)),
        }
    }
}

impl Stream for InitiatorBehavior {
    type Item = crate::InterfaceCommand<AnyMessage>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if let Poll::Ready(x) = self.outbound.futures.poll_next_unpin(cx) {
            if let Some(x) = x {
                return Poll::Ready(Some(x));
            }
        }

        if let Poll::Ready(x) = self.handshake.outbound.futures.poll_next_unpin(cx) {
            if let Some(x) = x {
                return Poll::Ready(Some(x));
            }
        }

        if let Poll::Ready(x) = self.housekeeping.poll_tick(cx) {
            println!("HOUSKEEPING TIMER");
        }

        Poll::Pending
    }
}

impl FusedStream for InitiatorBehavior {
    fn is_terminated(&self) -> bool {
        false
    }
}

impl Behavior for InitiatorBehavior {
    type Event = InitiatorEvent;
    type Command = InitiatorCommand;
    type PeerState = InitiatorState;
    type Message = AnyMessage;

    fn apply_io(
        &mut self,
        pid: &PeerId,
        state: &mut Self::PeerState,
        event: crate::InterfaceEvent<Self::Message>,
    ) -> Option<Self::Event> {
        let out = match &event {
            crate::InterfaceEvent::Connected(_) => {
                self.handshake.apply_io(pid, &mut state.handshake, event)
            }
            crate::InterfaceEvent::Recv(_, msg) => match msg {
                AnyMessage::Handshake(_) => {
                    self.handshake.apply_io(&pid, &mut state.handshake, event)
                }
                _ => None,
            },
            crate::InterfaceEvent::Sent(_, msg) => match msg {
                AnyMessage::Handshake(_) => {
                    self.handshake.apply_io(&pid, &mut state.handshake, event)
                }
                _ => None,
            },
            _ => None,
        };

        match out {
            Some(InitiatorEvent::PeerInitialized(pid)) => {
                state.connection = ConnectionState::Connected;

                Some(InitiatorEvent::PeerInitialized(pid))
            }
            Some(x) => Some(x),
            None => None,
        }
    }

    fn apply_cmd(&mut self, pid: &PeerId, state: &mut Self::PeerState, cmd: Self::Command) {
        match cmd {
            InitiatorCommand::IncludePeer(_) => {
                if self.stats.peers >= self.config.max_peers {
                    println!("max peers reached");
                    return;
                }

                state.priority = PeerPriority::Warm;
                self.stats.peers += 1;

                println!("requesting connection to {}", pid);
                self.outbound
                    .push_ready(crate::InterfaceCommand::Connect(pid.clone()));
            }
            _ => (),
        }
    }
}

pub struct ResponderBehavior;

pub struct ResponderState;

pub enum ResponderEvent {}

pub enum ResponderCommand {}
