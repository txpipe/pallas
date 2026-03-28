use futures::{Stream, StreamExt, stream::FusedStream};
use std::{collections::HashMap, task::Poll};

use crate::{
    Behavior, BehaviorOutput, Message as MessageTrait, OutboundQueue, PeerId, protocol as proto,
};

use super::{AcceptedVersion, AnyMessage, BlockRange, ConnectionState};

mod blockfetch;
mod chainsync;
mod connection;
mod discovery;
mod handshake;
mod keepalive;
mod promotion;

pub use blockfetch::*;
pub use chainsync::*;
pub use connection::*;
pub use discovery::*;
pub use handshake::*;
pub use keepalive::*;
pub use promotion::*;

/// A visitor trait that allows sub-behaviors to react to peer lifecycle events.
///
/// Each method is called by the initiator behavior at the appropriate point
/// in a peer's lifecycle. Default implementations are no-ops.
pub trait PeerVisitor {
    /// Called when a TCP connection to the peer is established.
    #[allow(unused_variables)]
    fn visit_connected(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    /// Called when the peer has been disconnected.
    #[allow(unused_variables)]
    fn visit_disconnected(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    /// Called when an error occurred on the peer's connection.
    #[allow(unused_variables)]
    fn visit_errored(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    /// Called when a new peer has been discovered.
    #[allow(unused_variables)]
    fn visit_discovered(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    /// Called when a message has been received from the peer.
    #[allow(unused_variables)]
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    /// Called after a message has been sent to the peer.
    #[allow(unused_variables)]
    fn visit_outbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    /// Called when a peer's state has been modified by a tag function.
    #[allow(unused_variables)]
    fn visit_tagged(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // default implementation does nothing
    }

    /// Called during periodic housekeeping for each tracked peer.
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

/// The promotion level of a peer, controlling which mini-protocols are active.
#[derive(PartialEq, Debug, Default, Copy, Clone)]
pub enum PromotionTag {
    /// Peer is known but not connected.
    #[default]
    Cold,
    /// Peer is connected and performing basic protocols (handshake, keepalive).
    Warm,
    /// Peer is fully active with all mini-protocols.
    Hot,
    /// Peer has been banned and will not be connected.
    Banned,
}

/// The per-peer state tracked by the initiator behavior, including connection
/// status and all mini-protocol state machines.
#[derive(Default, Debug)]
pub struct InitiatorState {
    pub(crate) connection: ConnectionState,
    pub(crate) promotion: PromotionTag,
    pub(crate) handshake: proto::handshake::State<proto::handshake::n2n::VersionData>,
    pub(crate) keepalive: proto::keepalive::State,
    pub(crate) peersharing: proto::peersharing::State,
    pub(crate) blockfetch: proto::blockfetch::State,
    pub(crate) chainsync: proto::chainsync::State<proto::chainsync::HeaderContent>,
    pub(crate) tx_submission: proto::txsubmission::State,
    pub(crate) violation: bool,
    pub(crate) error_count: u32,
    pub(crate) continue_sync: bool,
}

impl InitiatorState {
    /// Creates a new initiator state with default values for all protocols.
    pub fn new() -> Self {
        InitiatorState {
            connection: ConnectionState::default(),
            promotion: PromotionTag::default(),
            handshake: proto::handshake::State::default(),
            keepalive: proto::keepalive::State::default(),
            peersharing: proto::peersharing::State::default(),
            blockfetch: proto::blockfetch::State::default(),
            chainsync: crate::protocol::chainsync::State::default(),
            tx_submission: crate::protocol::txsubmission::State::default(),
            violation: false,
            error_count: 0,
            continue_sync: false,
        }
    }

    /// Returns true if the handshake has completed and mini-protocols are active.
    pub fn is_initialized(&self) -> bool {
        matches!(self.connection, ConnectionState::Initialized)
    }

    /// Returns the accepted version data if the handshake completed successfully.
    pub fn version(&self) -> Option<proto::handshake::n2n::VersionData> {
        match &self.handshake {
            proto::handshake::State::Done(proto::handshake::DoneState::Accepted(_, data)) => {
                Some(data.clone())
            }
            _ => None,
        }
    }

    /// Returns the current promotion level of this peer.
    pub fn promotion(&self) -> PromotionTag {
        self.promotion
    }

    /// Returns true if the negotiated version supports peer sharing.
    pub fn supports_peer_sharing(&self) -> bool {
        let val = self
            .version()
            .as_ref()
            .and_then(|v| v.peer_sharing)
            .unwrap_or(0);

        val > 0
    }

    /// Applies a message to the corresponding mini-protocol state machine.
    pub fn apply_msg(&mut self, msg: &AnyMessage) {
        match msg {
            AnyMessage::Handshake(msg) => {
                let result = self.handshake.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("handshake violation");
                    self.violation = true;
                    return;
                };

                self.handshake = new;
            }
            AnyMessage::KeepAlive(msg) => {
                let result = self.keepalive.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("keepalive violation");
                    self.violation = true;
                    return;
                };

                self.keepalive = new;
            }
            AnyMessage::PeerSharing(msg) => {
                let result = self.peersharing.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("peer sharing violation");
                    self.violation = true;
                    return;
                };

                self.peersharing = new;
            }
            AnyMessage::BlockFetch(msg) => {
                let result = self.blockfetch.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("block fetch violation");
                    self.violation = true;
                    return;
                };

                self.blockfetch = new;
            }
            AnyMessage::ChainSync(msg) => {
                let result = self.chainsync.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("chain sync violation");
                    self.violation = true;
                    return;
                };

                self.chainsync = new;
            }
            AnyMessage::TxSubmission(msg) => {
                let result = self.tx_submission.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("tx submission violation");
                    self.violation = true;
                    return;
                };

                self.tx_submission = new;
            }
        }
    }

    /// Resets the state back to its initial state, except for error count
    pub fn reset(&mut self) {
        self.connection = ConnectionState::default();
        self.promotion = PromotionTag::default();
        self.handshake = proto::handshake::State::default();
        self.keepalive = proto::keepalive::State::default();
        self.peersharing = proto::peersharing::State::default();
        self.blockfetch = proto::blockfetch::State::default();
        self.chainsync = proto::chainsync::State::default();
        self.tx_submission = proto::txsubmission::State::default();
        self.continue_sync = false;
        self.violation = false;
    }
}

/// A function that mutates an [`InitiatorState`], used for tagging operations
/// like banning or demoting peers.
pub type TagFn = fn(&mut InitiatorState);

/// Commands that can be sent to the initiator behavior from external code.
#[derive(Debug)]
pub enum InitiatorCommand {
    /// Add a new peer to be tracked and potentially connected.
    IncludePeer(PeerId),
    /// Trigger periodic housekeeping (peer promotion, discovery, etc.).
    Housekeeping,
    /// Begin chain synchronization from the given known points.
    StartSync(Vec<proto::Point>),
    /// Resume chain synchronization for a specific peer.
    ContinueSync(PeerId),
    /// Request a range of blocks to be fetched.
    RequestBlocks(BlockRange),
    /// Submit a transaction to a specific peer.
    SendTx(
        PeerId,
        proto::txsubmission::EraTxId,
        proto::txsubmission::EraTxBody,
    ),
    /// Ban a peer, preventing future connections.
    BanPeer(PeerId),
    /// Demote a peer back to cold status.
    DemotePeer(PeerId),
}

/// Events emitted by the initiator behavior to external consumers.
#[derive(Debug)]
pub enum InitiatorEvent {
    /// A peer completed the handshake and is ready for mini-protocols.
    PeerInitialized(PeerId, AcceptedVersion),
    /// An intersection point was found during chain-sync.
    IntersectionFound(PeerId, proto::Point, proto::chainsync::Tip),
    /// A new block header was received via chain-sync.
    BlockHeaderReceived(
        PeerId,
        proto::chainsync::HeaderContent,
        proto::chainsync::Tip,
    ),
    /// A rollback was received via chain-sync.
    RollbackReceived(PeerId, proto::Point, proto::chainsync::Tip),
    /// A block body was received via block-fetch.
    BlockBodyReceived(PeerId, proto::blockfetch::Body),
    /// The remote peer requested a transaction via tx-submission.
    TxRequested(PeerId, proto::txsubmission::EraTxId),
}

/// The main initiator behavior that orchestrates outbound Cardano connections.
///
/// Manages peer lifecycle (discovery, connection, promotion) and coordinates
/// all mini-protocol sub-behaviors (handshake, keepalive, chain-sync,
/// block-fetch, peer-sharing, discovery).
#[derive(Default)]
pub struct InitiatorBehavior {
    pub promotion: promotion::PromotionBehavior,
    pub connection: connection::ConnectionBehavior,
    pub handshake: handshake::HandshakeBehavior,
    pub keepalive: keepalive::KeepaliveBehavior,
    pub discovery: discovery::DiscoveryBehavior,
    pub blockfetch: blockfetch::BlockFetchBehavior,
    pub chainsync: chainsync::ChainSyncBehavior,
    pub peers: HashMap<PeerId, InitiatorState>,
    pub outbound: OutboundQueue<Self>,
}

macro_rules! all_visitors {
    ($self:ident, $pid:ident, $state:expr, $method:ident) => {
        $self.promotion.$method($pid, $state, &mut $self.outbound);
        $self.connection.$method($pid, $state, &mut $self.outbound);
        $self.handshake.$method($pid, $state, &mut $self.outbound);
        $self.keepalive.$method($pid, $state, &mut $self.outbound);
        $self.discovery.$method($pid, $state, &mut $self.outbound);
        $self.blockfetch.$method($pid, $state, &mut $self.outbound);
        $self.chainsync.$method($pid, $state, &mut $self.outbound);
    };
}

impl InitiatorBehavior {
    #[tracing::instrument(skip_all, fields(pid = %pid, channel = %msg.channel()))]
    /// Processes an inbound message from a peer, updating state and notifying visitors.
    pub fn on_inbound_msg(&mut self, pid: &PeerId, msg: &AnyMessage) {
        tracing::debug!(channel = msg.channel(), "new inbound message");

        self.peers.entry(pid.clone()).and_modify(|state| {
            state.apply_msg(msg);

            all_visitors!(self, pid, state, visit_inbound_msg);
        });
    }

    #[tracing::instrument(skip_all, fields(pid = %pid, channel = %msg.channel()))]
    /// Processes a confirmed outbound message to a peer, updating state and notifying visitors.
    pub fn on_outbound_msg(&mut self, pid: &PeerId, msg: &AnyMessage) {
        tracing::debug!(channel = msg.channel(), "new outbound message");

        self.peers.entry(pid.clone()).and_modify(|state| {
            state.apply_msg(msg);

            all_visitors!(self, pid, state, visit_outbound_msg);
        });
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_connected(&mut self, pid: &PeerId) {
        tracing::info!("connected");

        self.peers.entry(pid.clone()).and_modify(|state| {
            state.connection = ConnectionState::Connected;

            all_visitors!(self, pid, state, visit_connected);
        });
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_disconnected(&mut self, pid: &PeerId) {
        tracing::info!("disconnected");

        self.peers.entry(pid.clone()).and_modify(|state| {
            state.connection = ConnectionState::Disconnected;
            state.reset();

            all_visitors!(self, pid, state, visit_disconnected);
        });
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_errored(&mut self, pid: &PeerId) {
        tracing::error!("error");

        self.peers.entry(pid.clone()).and_modify(|state| {
            state.connection = ConnectionState::Errored;
            state.error_count += 1;

            all_visitors!(self, pid, state, visit_errored);
        });
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_tagged(&mut self, pid: &PeerId, tagger: TagFn) {
        tracing::debug!("tagged");

        self.peers.entry(pid.clone()).and_modify(|state| {
            tagger(state);

            all_visitors!(self, pid, state, visit_tagged);
        });
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_discovered(&mut self, pid: &PeerId) {
        let mut state = InitiatorState::new();

        all_visitors!(self, pid, &mut state, visit_discovered);

        self.peers.insert(pid.clone(), state);
    }

    fn move_discovered_into_promotion(&mut self) {
        let deficit = self.promotion.peer_deficit();

        if deficit == 0 {
            return;
        }

        let new = self.discovery.drain_new_peers(deficit);

        if new.is_empty() {
            tracing::trace!("no new peers discovered");
            return;
        }

        tracing::info!(deficit = deficit, new = new.len(), "discovered new peers",);

        for pid in new {
            if !self.peers.contains_key(&pid) {
                self.on_discovered(&pid);
            }
        }
    }

    #[tracing::instrument(skip_all)]
    fn housekeeping(&mut self) {
        for (pid, state) in self.peers.iter_mut() {
            all_visitors!(self, pid, state, visit_housekeeping);
        }

        self.move_discovered_into_promotion();
    }
}

impl Stream for InitiatorBehavior {
    type Item = BehaviorOutput<Self>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let poll = self.outbound.futures.poll_next_unpin(cx);

        match poll {
            Poll::Ready(Some(x)) => Poll::Ready(Some(x)),
            Poll::Ready(None) => Poll::Pending,
            Poll::Pending => Poll::Pending,
        }
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

    fn handle_io(&mut self, event: crate::InterfaceEvent<Self::Message>) {
        match &event {
            crate::InterfaceEvent::Connected(pid) => {
                self.on_connected(pid);
            }
            crate::InterfaceEvent::Disconnected(pid) => {
                self.on_disconnected(pid);
            }
            crate::InterfaceEvent::Recv(pid, msgs) => {
                for msg in msgs {
                    self.on_inbound_msg(pid, msg);
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

    fn execute(&mut self, cmd: Self::Command) {
        match cmd {
            InitiatorCommand::IncludePeer(pid) => {
                tracing::debug!("include peer command");
                self.on_discovered(&pid);
            }
            InitiatorCommand::StartSync(points) => {
                tracing::debug!("start sync command");
                self.chainsync.start(points);
            }
            InitiatorCommand::ContinueSync(pid) => {
                tracing::debug!("continue sync command");
                self.on_tagged(&pid, |state| state.continue_sync = true);
            }
            InitiatorCommand::RequestBlocks(range) => {
                tracing::debug!("request blocks command");
                self.blockfetch.enqueue(range);
            }
            InitiatorCommand::Housekeeping => {
                tracing::debug!("housekeeping command");
                self.housekeeping();
            }
            InitiatorCommand::BanPeer(pid) => {
                tracing::debug!("ban peer command");
                self.on_tagged(&pid, |state| state.promotion = PromotionTag::Banned);
            }
            InitiatorCommand::DemotePeer(pid) => {
                tracing::debug!("demote peer command");
                self.on_tagged(&pid, |state| state.promotion = PromotionTag::Cold);
            }
            InitiatorCommand::SendTx(..) => {
                tracing::warn!("SendTx not yet implemented");
            }
        }
    }
}
