//! Opinionated standard behavior for Cardano networks

use std::{collections::HashMap, task::Poll, time::Duration};

use futures::{Stream, StreamExt, stream::FusedStream};
use pallas_network::miniprotocols::{
    Agent, Point, blockfetch as blockfetch_proto, chainsync, handshake as handshake_proto,
    keepalive as keepalive_proto, peersharing as peersharing_proto, txsubmission,
};
use tokio::time::Interval;

use crate::{Behavior, BehaviorOutput, Message, OutboundQueue, PeerId};

mod blockfetch;
mod discovery;
mod handshake;
mod keepalive;
mod promotion;

#[derive(Debug, Clone)]
pub enum AnyMessage {
    Handshake(handshake_proto::Message<handshake_proto::n2n::VersionData>),
    KeepAlive(keepalive_proto::Message),
    ChainSync(chainsync::Message<chainsync::HeaderContent>),
    PeerSharing(peersharing_proto::Message),
    BlockFetch(blockfetch_proto::Message),
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

pub struct ChainSyncBehavior;

pub struct PeerSharingBehavior;

pub struct BlockFetchBehavior;

pub struct TxSubmissionBehavior;

pub type LastSeen = chrono::DateTime<chrono::Utc>;

#[derive(PartialEq, Debug)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Banned,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}

#[derive(Default, Debug)]
pub struct InitiatorState {
    connection: ConnectionState,
    handshake: handshake_proto::Client<handshake_proto::n2n::VersionData>,
    keepalive: keepalive_proto::Client,
    peersharing: peersharing_proto::Client,
    blockfetch: blockfetch_proto::Client,
    violation: bool,
}

impl InitiatorState {
    pub fn new() -> Self {
        InitiatorState {
            connection: ConnectionState::Disconnected,
            handshake: handshake_proto::Client::default(),
            keepalive: keepalive_proto::Client::default(),
            peersharing: peersharing_proto::Client::default(),
            blockfetch: blockfetch_proto::Client::default(),
            violation: false,
        }
    }

    pub fn needs_connection(&self) -> bool {
        matches!(self.connection, ConnectionState::Disconnected)
    }

    pub fn is_initialized(&self) -> bool {
        // TODO: handle rejected
        matches!(self.handshake.state(), handshake_proto::State::Done(_))
    }

    pub fn version(&self) -> Option<handshake_proto::n2n::VersionData> {
        match self.handshake.state() {
            handshake_proto::State::Done(handshake_proto::DoneState::Accepted(_, data)) => {
                Some(data.clone())
            }
            _ => None,
        }
    }

    pub fn supports_peer_sharing(&self) -> bool {
        self.version()
            .map(|v| v.peer_sharing.is_some())
            .unwrap_or(false)
    }

    pub fn apply_msg(&mut self, msg: &AnyMessage) {
        match msg {
            AnyMessage::Handshake(msg) => match self.handshake.apply(msg) {
                Ok(sm) => {
                    self.handshake = handshake_proto::Client::new(sm);
                }
                Err(_) => {
                    tracing::warn!("handshake violation");
                    self.violation = true;
                }
            },
            AnyMessage::KeepAlive(msg) => match self.keepalive.apply(msg) {
                Ok(sm) => {
                    self.keepalive = keepalive_proto::Client::new(sm);
                }
                Err(_) => {
                    tracing::warn!("keepalive violation");
                    self.violation = true;
                }
            },
            AnyMessage::PeerSharing(msg) => match self.peersharing.apply(msg) {
                Ok(sm) => {
                    self.peersharing = peersharing_proto::Client::new(sm);
                }
                Err(_) => {
                    tracing::warn!("peer sharing violation");
                    self.violation = true;
                }
            },
            AnyMessage::BlockFetch(msg) => match self.blockfetch.apply(msg) {
                Ok(sm) => {
                    self.blockfetch = blockfetch_proto::Client::new(sm);
                }
                Err(_) => {
                    tracing::warn!("block fetch violation");
                    self.violation = true;
                }
            },
            AnyMessage::ChainSync(_) => todo!(),
            AnyMessage::TxSubmission(_) => todo!(),
        }
    }
}

pub type BlockRange = (Point, Point);

pub enum InitiatorCommand {
    IncludePeer(PeerId),
    IntersectChain(PeerId, Point),
    RequestNextHeader(PeerId, Point),
    RequestBlockBatch(BlockRange, Option<PeerId>),
    SendTx(PeerId, txsubmission::EraTxId, txsubmission::EraTxBody),
}

#[derive(Debug)]
pub enum InitiatorEvent {
    PeerInitialized(PeerId, handshake_proto::n2n::VersionData),
    BlockHeaderReceived(PeerId, chainsync::HeaderContent),
    BlockBodyReceived(PeerId, blockfetch_proto::Body),
    TxRequested(PeerId, txsubmission::EraTxId),
}

pub struct InitiatorBehavior {
    peers: HashMap<PeerId, InitiatorState>,
    promotion: promotion::PromotionBehavior,
    handshake: handshake::HandshakeBehavior,
    keepalive: keepalive::KeepaliveBehavior,
    discovery: discovery::DiscoveryBehavior,
    blockfetch: blockfetch::BlockFetchBehavior,
    outbound: OutboundQueue<Self>,
    housekeeping: Interval,
}

impl InitiatorBehavior {
    /// Define a peer to use for a given command.
    ///
    /// Commands that require a specific peer can either provide it explicitly
    /// or let the behavior select a random hot peer. This method will
    /// handle that logic for you.
    ///
    /// If the list of hot peers is empty, this method will return `None`.
    pub fn define_peer(&mut self, pid: Option<PeerId>) -> Option<PeerId> {
        if let Some(pid) = pid {
            return Some(pid);
        }

        tracing::debug!("no peer provided, selecting random hot peer");

        if let Some(pid) = self.promotion.select_random_hot_peer() {
            return Some(pid.clone());
        }

        tracing::debug!("no hot peers available");

        None
    }

    pub fn visit_updated_peer(&mut self, pid: &PeerId, state: &mut InitiatorState) {
        self.handshake
            .visit_updated_peer(pid, state, &mut self.outbound);

        self.discovery
            .visit_updated_peer(pid, state, &mut self.outbound);

        self.promotion
            .visit_updated_peer(pid, state, &mut self.outbound);

        self.blockfetch
            .visit_updated_peer(pid, state, &mut self.outbound);
    }

    #[tracing::instrument(skip(self, msg))]
    pub fn on_msg(&mut self, pid: &PeerId, msg: &AnyMessage) {
        let entry = self.peers.remove(pid);

        if let Some(mut state) = entry {
            state.apply_msg(msg);

            self.visit_updated_peer(pid, &mut state);

            self.peers.insert(pid.clone(), state);
        }
    }

    #[tracing::instrument(skip(self))]
    fn on_connected(&mut self, pid: &PeerId) {
        let entry = self.peers.remove(pid);

        if let Some(mut state) = entry {
            state.connection = ConnectionState::Connected;

            self.visit_updated_peer(pid, &mut state);

            self.peers.insert(pid.clone(), state);
        }
    }

    #[tracing::instrument(skip(self))]
    fn on_disconnected(&mut self, pid: &PeerId) {
        let entry = self.peers.remove(pid);

        if let Some(mut state) = entry {
            state.connection = ConnectionState::Disconnected;

            self.visit_updated_peer(pid, &mut state);

            self.peers.insert(pid.clone(), state);
        }
    }

    fn on_discovered(&mut self, pid: &PeerId) {
        self.promotion.on_peer_discovered(pid);

        self.peers.insert(pid.clone(), InitiatorState::new());
    }

    fn housekeeping(&mut self) {
        for (pid, peer) in self.peers.iter_mut() {
            self.promotion
                .on_peer_housekeeping(pid, peer, &mut self.outbound);

            self.keepalive
                .on_peer_housekeeping(pid, peer, &mut self.outbound);
        }

        for pid in self.discovery.take_peers() {
            self.on_discovered(&pid);
        }
    }
}

impl Default for InitiatorBehavior {
    fn default() -> Self {
        Self {
            peers: Default::default(),
            promotion: promotion::PromotionBehavior::default(),
            handshake: handshake::HandshakeBehavior::default(),
            keepalive: keepalive::KeepaliveBehavior::default(),
            discovery: discovery::DiscoveryBehavior::default(),
            blockfetch: blockfetch::BlockFetchBehavior::default(),
            outbound: Default::default(),
            housekeeping: tokio::time::interval(Duration::from_millis(10_000)),
        }
    }
}

impl Stream for InitiatorBehavior {
    type Item = BehaviorOutput<Self>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if self.housekeeping.poll_tick(cx).is_ready() {
            self.housekeeping();
        }

        if let Poll::Ready(Some(x)) = self.outbound.futures.poll_next_unpin(cx) {
            return Poll::Ready(Some(x));
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

    fn apply_io(&mut self, event: crate::InterfaceEvent<Self::Message>) {
        match &event {
            crate::InterfaceEvent::Connected(pid) => {
                self.on_connected(pid);
            }
            crate::InterfaceEvent::Disconnected(pid) => {
                self.on_disconnected(pid);
            }
            crate::InterfaceEvent::Recv(pid, msg) => {
                self.on_msg(pid, msg);
            }
            crate::InterfaceEvent::Sent(pid, msg) => {
                self.on_msg(pid, msg);
            }
            _ => (),
        }
    }

    fn apply_cmd(&mut self, cmd: Self::Command) {
        match cmd {
            InitiatorCommand::IncludePeer(pid) => {
                self.on_discovered(&pid);
            }
            InitiatorCommand::IntersectChain(pid, _point) => {
                tracing::info!(%pid, "intersecting chain");
            }
            InitiatorCommand::RequestBlockBatch(range, pid) => {
                tracing::info!(?pid, "requesting block batch");

                let Some(pid) = self.define_peer(pid) else {
                    tracing::error!("can't request block without a hot peer");
                    return;
                };

                self.blockfetch
                    .request_block_batch(&pid, range, &mut self.outbound);
            }
            _ => (),
        }
    }
}

pub struct ResponderBehavior;

pub struct ResponderState;

pub enum ResponderEvent {}

pub enum ResponderCommand {}
