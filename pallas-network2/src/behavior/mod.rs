//! Opinionated standard behavior for Cardano networks

use std::{collections::HashMap, task::Poll, time::Duration};

use futures::{Stream, StreamExt, stream::FusedStream};
use pallas_codec::{Fragment, minicbor};
use pallas_network::miniprotocols::{
    Agent, Point, blockfetch as blockfetch_proto, chainsync, handshake as handshake_proto,
    keepalive as keepalive_proto, peersharing as peersharing_proto, txsubmission,
};
use tokio::time::Interval;

use crate::{Behavior, BehaviorOutput, Channel, Message, OutboundQueue, Payload, PeerId};

mod blockfetch;
mod connection;
mod discovery;
mod handshake;
mod keepalive;
mod promotion;

pub trait PeerVisitor {
    #[allow(unused_variables)]
    fn visit_connected(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    #[allow(unused_variables)]
    fn visit_disconnected(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    #[allow(unused_variables)]
    fn visit_errored(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    #[allow(unused_variables)]
    fn visit_discovered(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    #[allow(unused_variables)]
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    #[allow(unused_variables)]
    fn visit_outbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    #[allow(unused_variables)]
    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }
}

#[derive(Debug, Clone)]
pub enum AnyMessage {
    Handshake(handshake_proto::Message<handshake_proto::n2n::VersionData>),
    KeepAlive(keepalive_proto::Message),
    ChainSync(chainsync::Message<chainsync::HeaderContent>),
    PeerSharing(peersharing_proto::Message),
    BlockFetch(blockfetch_proto::Message),
    TxSubmission(txsubmission::Message<txsubmission::EraTxId, txsubmission::EraTxBody>),
}

fn try_decode_message<T: Fragment>(buffer: &mut Vec<u8>) -> Option<T> {
    let mut decoder = minicbor::Decoder::new(buffer);
    let maybe_msg: Result<T, _> = decoder.decode();

    match maybe_msg {
        Ok(msg) => {
            let new_pos = decoder.position();
            buffer.drain(0..new_pos);
            Some(msg)
        }
        Err(err) if err.is_end_of_input() => None,
        Err(err) => {
            tracing::error!(?err);
            None
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

    fn from_payload(channel: Channel, payload: &mut Payload) -> Option<Self> {
        let channel = channel ^ 0x8000;

        match channel {
            pallas_network::miniprotocols::PROTOCOL_N2N_HANDSHAKE => {
                try_decode_message(payload).map(AnyMessage::Handshake)
            }
            pallas_network::miniprotocols::PROTOCOL_N2N_KEEP_ALIVE => {
                try_decode_message(payload).map(AnyMessage::KeepAlive)
            }
            pallas_network::miniprotocols::PROTOCOL_N2N_CHAIN_SYNC => {
                try_decode_message(payload).map(AnyMessage::ChainSync)
            }
            pallas_network::miniprotocols::PROTOCOL_N2N_PEER_SHARING => {
                try_decode_message(payload).map(AnyMessage::PeerSharing)
            }
            pallas_network::miniprotocols::PROTOCOL_N2N_BLOCK_FETCH => {
                try_decode_message(payload).map(AnyMessage::BlockFetch)
            }
            pallas_network::miniprotocols::PROTOCOL_N2N_TX_SUBMISSION => {
                try_decode_message(payload).map(AnyMessage::TxSubmission)
            }
            x => unimplemented!("unsupported channel: {}", x),
        }
    }

    fn into_payload(self) -> (Channel, Payload) {
        let channel = self.channel();
        let payload = self.payload();

        (channel, payload)
    }
}

pub struct ChainSyncBehavior;

pub struct PeerSharingBehavior;

pub struct BlockFetchBehavior;

pub struct TxSubmissionBehavior;

pub type LastSeen = chrono::DateTime<chrono::Utc>;

#[derive(PartialEq, Debug, Default)]
pub enum ConnectionState {
    #[default]
    New,
    Connecting,
    Connected,
    Initialized,
    Disconnected,
    Errored,
}

#[derive(PartialEq, Debug, Default, Copy, Clone)]
pub enum Promotion {
    #[default]
    Cold,
    Warm,
    Hot,
    Banned,
}

#[derive(Default, Debug)]
pub struct InitiatorState {
    connection: ConnectionState,
    promotion: Promotion,
    handshake: handshake_proto::Client<handshake_proto::n2n::VersionData>,
    keepalive: keepalive_proto::Client,
    peersharing: peersharing_proto::Client,
    blockfetch: blockfetch_proto::Client,
    violation: bool,
    error_count: u32,
}

impl InitiatorState {
    pub fn new() -> Self {
        InitiatorState {
            connection: ConnectionState::default(),
            promotion: Promotion::default(),
            handshake: handshake_proto::Client::default(),
            keepalive: keepalive_proto::Client::default(),
            peersharing: peersharing_proto::Client::default(),
            blockfetch: blockfetch_proto::Client::default(),
            violation: false,
            error_count: 0,
        }
    }

    pub fn is_initialized(&self) -> bool {
        matches!(self.connection, ConnectionState::Initialized)
    }

    pub fn version(&self) -> Option<handshake_proto::n2n::VersionData> {
        match self.handshake.state() {
            handshake_proto::State::Done(handshake_proto::DoneState::Accepted(_, data)) => {
                Some(data.clone())
            }
            _ => None,
        }
    }

    pub fn promotion(&self) -> Promotion {
        self.promotion
    }

    pub fn supports_peer_sharing(&self) -> bool {
        let val = self
            .version()
            .as_ref()
            .and_then(|v| v.peer_sharing)
            .unwrap_or(0);

        val > 0
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

    /// Resets the state back to its initial state, except for error count
    pub fn reset(&mut self) {
        self.connection = ConnectionState::default();
        self.promotion = Promotion::default();
        self.handshake = handshake_proto::Client::default();
        self.keepalive = keepalive_proto::Client::default();
        self.peersharing = peersharing_proto::Client::default();
        self.blockfetch = blockfetch_proto::Client::default();
        self.violation = false;
    }
}

pub type BlockRange = (Point, Point);

pub enum InitiatorCommand {
    IncludePeer(PeerId),
    IntersectChain(PeerId, Point),
    RequestNextHeader(PeerId, Point),
    RequestBlockBatch(BlockRange),
    SendTx(PeerId, txsubmission::EraTxId, txsubmission::EraTxBody),
}

pub type AcceptedVersion = (u64, handshake_proto::n2n::VersionData);

#[derive(Debug)]
pub enum InitiatorEvent {
    PeerInitialized(PeerId, AcceptedVersion),
    BlockHeaderReceived(PeerId, chainsync::HeaderContent),
    BlockBodyReceived(PeerId, blockfetch_proto::Body),
    TxRequested(PeerId, txsubmission::EraTxId),
}

pub struct InitiatorBehavior {
    peers: HashMap<PeerId, InitiatorState>,
    promotion: promotion::PromotionBehavior,
    connection: connection::ConnectionBehavior,
    handshake: handshake::HandshakeBehavior,
    keepalive: keepalive::KeepaliveBehavior,
    discovery: discovery::DiscoveryBehavior,
    blockfetch: blockfetch::BlockFetchBehavior,
    outbound: OutboundQueue<Self>,
    housekeeping: Interval,
}

macro_rules! all_visitors {
    ($self:ident, $pid:ident, $state:expr, $method:ident) => {
        $self.promotion.$method($pid, $state, &mut $self.outbound);
        $self.connection.$method($pid, $state, &mut $self.outbound);
        $self.handshake.$method($pid, $state, &mut $self.outbound);
        $self.keepalive.$method($pid, $state, &mut $self.outbound);
        $self.discovery.$method($pid, $state, &mut $self.outbound);
        $self.blockfetch.$method($pid, $state, &mut $self.outbound);
    };
}

impl InitiatorBehavior {
    #[tracing::instrument(skip_all, fields(pid = %pid, channel = %msg.channel()))]
    pub fn on_inbound_msg(&mut self, pid: &PeerId, msg: &AnyMessage) {
        let entry = self.peers.remove(pid);

        if let Some(mut state) = entry {
            state.apply_msg(msg);

            all_visitors!(self, pid, &mut state, visit_inbound_msg);

            self.peers.insert(pid.clone(), state);
        }
    }

    #[tracing::instrument(skip_all, fields(pid = %pid, channel = %msg.channel()))]
    pub fn on_outbound_msg(&mut self, pid: &PeerId, msg: &AnyMessage) {
        tracing::info!(channel = msg.channel(), "new outbound message");

        let entry = self.peers.remove(pid);

        if let Some(mut state) = entry {
            state.apply_msg(msg);

            all_visitors!(self, pid, &mut state, visit_outbound_msg);

            self.peers.insert(pid.clone(), state);
        }
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_connected(&mut self, pid: &PeerId) {
        tracing::info!("connected");

        let entry = self.peers.remove(pid);

        if let Some(mut state) = entry {
            state.connection = ConnectionState::Connected;

            all_visitors!(self, pid, &mut state, visit_connected);

            self.peers.insert(pid.clone(), state);
        }
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_disconnected(&mut self, pid: &PeerId) {
        tracing::info!("disconnected");

        let entry = self.peers.remove(pid);

        if let Some(mut state) = entry {
            state.connection = ConnectionState::Disconnected;
            state.reset();

            all_visitors!(self, pid, &mut state, visit_disconnected);

            self.peers.insert(pid.clone(), state);
        }
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_errored(&mut self, pid: &PeerId) {
        tracing::error!("error");

        let entry = self.peers.remove(pid);

        if let Some(mut state) = entry {
            state.connection = ConnectionState::Errored;
            state.error_count += 1;

            all_visitors!(self, pid, &mut state, visit_errored);

            self.peers.insert(pid.clone(), state);
        }
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_discovered(&mut self, pid: &PeerId) {
        let mut state = InitiatorState::new();

        all_visitors!(self, pid, &mut state, visit_discovered);

        self.peers.insert(pid.clone(), state);
    }

    #[tracing::instrument(skip_all)]
    fn housekeeping(&mut self) {
        for (pid, state) in self.peers.iter_mut() {
            all_visitors!(self, pid, state, visit_housekeeping);
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
            connection: connection::ConnectionBehavior::default(),
            handshake: handshake::HandshakeBehavior::default(),
            keepalive: keepalive::KeepaliveBehavior::default(),
            discovery: discovery::DiscoveryBehavior::default(),
            blockfetch: blockfetch::BlockFetchBehavior::default(),
            outbound: Default::default(),
            housekeeping: tokio::time::interval(Duration::from_millis(3_000)),
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
            crate::InterfaceEvent::Recv(pid, msgs) => {
                for msg in msgs {
                    self.on_inbound_msg(pid, &msg);
                }
            }
            crate::InterfaceEvent::Sent(pid, msg) => {
                self.on_outbound_msg(pid, msg);
            }
            crate::InterfaceEvent::Error(pid, _) => {
                self.on_errored(pid);
            }
            crate::InterfaceEvent::Idle => {
                self.housekeeping();
            }
        }
    }

    fn apply_cmd(&mut self, cmd: Self::Command) {
        match cmd {
            InitiatorCommand::IncludePeer(pid) => {
                self.on_discovered(&pid);
            }
            InitiatorCommand::IntersectChain(pid, _point) => {
                tracing::info!(%pid, "TODO: intersecting chain");
            }
            InitiatorCommand::RequestBlockBatch(range) => {
                tracing::info!("enqueueing block batch");
                self.blockfetch.enqueue(range);
            }
            _ => (),
        }
    }
}

pub struct ResponderBehavior;

pub struct ResponderState;

pub enum ResponderEvent {}

pub enum ResponderCommand {}
