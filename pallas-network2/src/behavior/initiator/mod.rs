use futures::{Stream, StreamExt, stream::FusedStream};
use std::{collections::HashMap, task::Poll};

use crate::{
    Behavior, BehaviorOutput, Message as MessageTrait, OutboundQueue, PeerId, protocol as proto,
};

use super::{AcceptedVersion, AnyMessage, BlockRange, ConnectionState, unsent::UnsentRequests};

mod blockfetch;
mod chainsync;
mod connection;
mod discovery;
mod handshake;
mod keepalive;
mod leiosfetch;
mod leiosnotify;
mod promotion;

pub use blockfetch::*;
pub use chainsync::*;
pub use connection::*;
pub use discovery::*;
pub use handshake::*;
pub use keepalive::*;
pub use leiosfetch::*;
pub use leiosnotify::*;
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
    pub(crate) leios_notify: proto::leiosnotify::State,
    pub(crate) leios_fetch: proto::leiosfetch::State,
    pub(crate) violation: bool,
    pub(crate) error_count: u32,
    pub(crate) continue_sync: bool,
    pub(crate) sync_next_deferred: bool,
    pub(crate) unsent: UnsentRequests,
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
            leios_notify: proto::leiosnotify::State::default(),
            leios_fetch: proto::leiosfetch::State::default(),
            violation: false,
            error_count: 0,
            continue_sync: false,
            sync_next_deferred: false,
            unsent: UnsentRequests::default(),
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

    /// Returns the negotiated N2N protocol version, if the handshake completed.
    pub fn accepted_version(&self) -> Option<u64> {
        super::accepted_version(&self.handshake)
    }

    /// Returns true if the negotiated version carries the Leios overlay.
    pub fn supports_leios(&self) -> bool {
        super::supports_leios(&self.handshake)
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
            AnyMessage::LeiosNotify(msg) => {
                let result = self.leios_notify.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("leios notify violation");
                    self.violation = true;
                    return;
                };

                self.leios_notify = new;
            }
            AnyMessage::LeiosFetch(msg) => {
                let result = self.leios_fetch.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("leios fetch violation");
                    self.violation = true;
                    return;
                };

                self.leios_fetch = new;
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
        self.leios_notify = proto::leiosnotify::State::default();
        self.leios_fetch = proto::leiosfetch::State::default();
        self.continue_sync = false;
        self.sync_next_deferred = false;
        self.violation = false;
        self.unsent = UnsentRequests::default();
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
    /// Request a complete EB body from a peer (leios-fetch).
    FetchEb(PeerId, proto::EbId),
    /// Request a subset of an EB's transactions from a peer (leios-fetch).
    FetchEbTxs(PeerId, proto::EbId, proto::leiosfetch::Bitmaps),
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
    /// An EB announcement or offer was received via leios-notify.
    EbNotification(PeerId, proto::leiosnotify::Notification),
    /// An EB body or transactions were received via leios-fetch, for the given EB.
    EbFetched(PeerId, proto::EbId, proto::leiosfetch::Response),
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
    pub leiosnotify: leiosnotify::LeiosNotifyBehavior,
    pub leiosfetch: leiosfetch::LeiosFetchBehavior,
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
        $self.leiosnotify.$method($pid, $state, &mut $self.outbound);
        $self.leiosfetch.$method($pid, $state, &mut $self.outbound);
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
            state.clear_unsent(msg);
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

            all_visitors!(self, pid, state, visit_disconnected);

            state.reset();
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

    /// Puts a queued leios-fetch request on the wire for `pid` straight away,
    /// rather than waiting for the next housekeeping tick.
    fn serve_leios_fetch(&mut self, pid: &PeerId) {
        let Self {
            leiosfetch,
            peers,
            outbound,
            ..
        } = self;

        if let Some(state) = peers.get_mut(pid) {
            leiosfetch.serve_next(pid, state, outbound);
        }
    }

    /// Sends one queued block range to each available peer while ranges remain.
    fn serve_block_fetch(&mut self) {
        let Self {
            blockfetch,
            peers,
            outbound,
            ..
        } = self;

        for (pid, state) in peers.iter_mut() {
            blockfetch.serve_next(pid, state, outbound);
        }
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
                self.serve_block_fetch();
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
            InitiatorCommand::FetchEb(pid, point) => {
                tracing::debug!("fetch eb command");
                self.leiosfetch
                    .enqueue(pid.clone(), leiosfetch::FetchRequest::Block(point));
                self.serve_leios_fetch(&pid);
            }
            InitiatorCommand::FetchEbTxs(pid, point, bitmaps) => {
                tracing::debug!("fetch eb txs command");
                self.leiosfetch.enqueue(
                    pid.clone(),
                    leiosfetch::FetchRequest::BlockTxs(point, bitmaps),
                );
                self.serve_leios_fetch(&pid);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        AnyCbor, MAINNET_MAGIC, Point, blockfetch as bf, chainsync as cs, handshake, keepalive,
        leiosfetch as lf, leiosnotify as ln, peersharing,
    };
    use crate::testing::BehaviorOutputExt;
    use crate::{InterfaceError, InterfaceEvent};
    use futures::StreamExt;
    use std::collections::HashMap;
    use std::net::Ipv4Addr;

    fn drain_outputs(behavior: &mut InitiatorBehavior) -> Vec<BehaviorOutput<InitiatorBehavior>> {
        let mut outputs = Vec::new();
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        while let std::task::Poll::Ready(Some(output)) = behavior.poll_next_unpin(&mut cx) {
            outputs.push(output);
        }

        outputs
    }

    fn complete_handshake(
        behavior: &mut InitiatorBehavior,
        pid: &PeerId,
    ) -> Vec<BehaviorOutput<InitiatorBehavior>> {
        let version_data =
            handshake::n2n::VersionData::new(MAINNET_MAGIC, false, Some(1), Some(false));
        let mut values = HashMap::new();
        values.insert(13u64, version_data.clone());
        let version_table = handshake::VersionTable { values };

        let propose = AnyMessage::Handshake(handshake::Message::Propose(version_table));
        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), propose));
        drain_outputs(behavior);

        let accept = AnyMessage::Handshake(handshake::Message::Accept(13, version_data));
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![accept]));
        drain_outputs(behavior)
    }

    /// Completes a handshake negotiating a Leios-capable version (15).
    fn complete_handshake_leios(
        behavior: &mut InitiatorBehavior,
        pid: &PeerId,
    ) -> Vec<BehaviorOutput<InitiatorBehavior>> {
        let version_data =
            handshake::n2n::VersionData::new(MAINNET_MAGIC, false, Some(1), Some(false));
        let mut values = HashMap::new();
        values.insert(15u64, version_data.clone());
        let version_table = handshake::VersionTable { values };

        let propose = AnyMessage::Handshake(handshake::Message::Propose(version_table));
        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), propose));
        drain_outputs(behavior);

        let accept = AnyMessage::Handshake(handshake::Message::Accept(15, version_data));
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![accept]));
        drain_outputs(behavior)
    }

    // ---- Kept: genuinely cross-cutting tests ----

    #[tokio::test]
    async fn banned_peer_not_reconnected() {
        // Composition: violation flag → promotion ban → connection guard
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(1);

        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        drain_outputs(&mut behavior);

        let bad_msg = AnyMessage::KeepAlive(keepalive::Message::ResponseKeepAlive(42));
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![bad_msg]));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        behavior.handle_io(InterfaceEvent::Disconnected(pid.clone()));
        drain_outputs(&mut behavior);

        for _ in 0..10 {
            behavior.execute(InitiatorCommand::Housekeeping);
            let outputs = drain_outputs(&mut behavior);
            assert!(!outputs.has_connect_for(&pid));
        }
    }

    #[tokio::test]
    async fn demote_peer_returns_to_cold() {
        // Composition: handshake → promotion hot → demote tag → connection disconnect
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(2);

        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        drain_outputs(&mut behavior);
        complete_handshake(&mut behavior, &pid);

        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);
        assert!(behavior.promotion.hot_peers.contains(&pid));

        behavior.execute(InitiatorCommand::DemotePeer(pid.clone()));
        drain_outputs(&mut behavior);

        let state = behavior.peers.get(&pid).unwrap();
        assert_eq!(state.promotion, PromotionTag::Cold);

        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);
        assert!(outputs.has_disconnect_for(&pid));
    }

    #[tokio::test]
    async fn error_count_persists_across_disconnect() {
        // Composition: on_errored increments → on_disconnected resets but preserves
        //              error_count → promotion bans on threshold
        tokio::time::pause();

        let mut behavior = InitiatorBehavior {
            promotion: PromotionBehavior::new(PromotionConfig {
                max_error_count: 2,
                ..PromotionConfig::default()
            }),
            ..Default::default()
        };
        let pid = PeerId::test(3);

        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        for _ in 0..2 {
            behavior.handle_io(InterfaceEvent::Error(
                pid.clone(),
                InterfaceError::Other("err".into()),
            ));
            behavior.execute(InitiatorCommand::Housekeeping);
            drain_outputs(&mut behavior);
            behavior.handle_io(InterfaceEvent::Disconnected(pid.clone()));
            drain_outputs(&mut behavior);
        }

        assert!(!behavior.promotion.banned_peers.contains(&pid));

        behavior.handle_io(InterfaceEvent::Error(
            pid.clone(),
            InterfaceError::Other("err".into()),
        ));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        assert!(behavior.promotion.banned_peers.contains(&pid));
    }

    // ---- New: composition tests ----

    #[tokio::test]
    async fn full_peer_lifecycle_include_to_chainsync() {
        // Composition: promotion → connection → handshake → promotion (warm→hot) → chainsync
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(10);

        // Start chainsync so the behavior will initiate it for hot peers
        behavior.execute(InitiatorCommand::StartSync(vec![Point::Origin]));

        // Include peer → housekeeping promotes cold→warm and connects
        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);

        assert!(behavior.promotion.warm_peers.contains(&pid));
        assert!(outputs.has_connect_for(&pid));

        // Connected → handshake proposes
        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        let outputs = drain_outputs(&mut behavior);
        assert!(
            outputs
                .has_send(|m| matches!(m, AnyMessage::Handshake(handshake::Message::Propose(_))))
        );

        // Complete handshake → Initialized
        complete_handshake(&mut behavior, &pid);

        // Housekeeping promotes warm→hot, chainsync starts FindIntersect
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);

        assert!(behavior.promotion.hot_peers.contains(&pid));
        assert!(
            outputs.has_send(|m| matches!(m, AnyMessage::ChainSync(cs::Message::FindIntersect(_)))),
            "chainsync should start for hot initialized peer"
        );
    }

    #[tokio::test]
    async fn housekeeping_promotes_and_connects_in_same_pass() {
        // Composition: visitor ordering — promotion runs before connection in all_visitors!
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(11);

        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));

        // Single housekeeping call should both promote cold→warm AND issue Connect
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);

        assert!(
            behavior.promotion.warm_peers.contains(&pid),
            "peer should be promoted to warm"
        );
        assert!(
            outputs.has_connect_for(&pid),
            "Connect should be issued in the same housekeeping pass"
        );
    }

    #[tokio::test]
    async fn discovery_feeds_into_promotion() {
        // Composition: discovery accumulates peers → housekeeping drains → promotion adds to cold
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let seed_pid = PeerId::test(12);

        // Include and fully initialize a seed peer with peer-sharing support
        behavior.execute(InitiatorCommand::IncludePeer(seed_pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        behavior.handle_io(InterfaceEvent::Connected(seed_pid.clone()));
        drain_outputs(&mut behavior);
        complete_handshake(&mut behavior, &seed_pid);
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        // Simulate peersharing response with 2 new peers
        let share_response = AnyMessage::PeerSharing(peersharing::Message::SharePeers(vec![
            peersharing::PeerAddress::V4(Ipv4Addr::new(192, 168, 1, 1), 3000),
            peersharing::PeerAddress::V4(Ipv4Addr::new(192, 168, 1, 2), 3001),
        ]));

        // We need the seed peer's peersharing state to be in the right state first.
        // Simulate the outbound ShareRequest being sent (to move state to Busy)
        let share_req = AnyMessage::PeerSharing(peersharing::Message::ShareRequest(10));
        behavior.handle_io(InterfaceEvent::Sent(seed_pid.clone(), share_req));
        drain_outputs(&mut behavior);

        // Now receive the response
        behavior.handle_io(InterfaceEvent::Recv(seed_pid.clone(), vec![share_response]));
        drain_outputs(&mut behavior);

        // Housekeeping should move discovered peers into promotion
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        // The discovered peers should now be tracked
        let discovered_1 = PeerId {
            host: "192.168.1.1".to_string(),
            port: 3000,
        };
        let discovered_2 = PeerId {
            host: "192.168.1.2".to_string(),
            port: 3001,
        };

        assert!(
            behavior.peers.contains_key(&discovered_1),
            "discovered peer 1 should be tracked after housekeeping"
        );
        assert!(
            behavior.peers.contains_key(&discovered_2),
            "discovered peer 2 should be tracked after housekeeping"
        );
    }

    #[tokio::test]
    async fn violation_bans_and_disconnects() {
        // Composition: apply_msg sets violation → promotion bans → connection disconnects
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(13);

        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        drain_outputs(&mut behavior);

        // Protocol violation
        let bad_msg = AnyMessage::KeepAlive(keepalive::Message::ResponseKeepAlive(42));
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![bad_msg]));

        // Housekeeping should both ban (promotion) AND disconnect (connection)
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);

        assert!(
            behavior.promotion.banned_peers.contains(&pid),
            "promotion should ban the violating peer"
        );
        assert!(
            outputs.has_disconnect_for(&pid),
            "connection should disconnect the banned peer"
        );
    }

    #[tokio::test]
    async fn blockfetch_requires_initialized_and_idle() {
        // Composition: handshake state gates blockfetch dispatch
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(14);

        let range = (Point::Origin, Point::new(100, vec![0xAA; 32]));
        behavior.blockfetch.enqueue(range.clone());

        // Include peer, promote to warm, connect (but NOT handshaked)
        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        drain_outputs(&mut behavior);

        // Housekeeping — peer is Connected but not Initialized, so no RequestRange
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);
        assert!(
            !outputs
                .has_send(|m| matches!(m, AnyMessage::BlockFetch(bf::Message::RequestRange(_)))),
            "should NOT send RequestRange before handshake"
        );

        let accepted = complete_handshake(&mut behavior, &pid);
        let sent: Vec<_> = accepted
            .sends()
            .filter(|&(p, m)| *p == pid && is_any_range(m))
            .map(|(_, m)| m.clone())
            .collect();
        assert_eq!(
            sent.len(),
            1,
            "should send RequestRange when the handshake completes"
        );
        assert!(
            is_range(&sent[0], &range),
            "the queued range goes on the handshake, got {sent:?}"
        );
        assert!(!behavior.peers[&pid].violation, "no violation");
    }

    #[tokio::test]
    async fn leios_notify_and_fetch_flow() {
        // Composition: handshake negotiates v15 → supports_leios → notify pull
        // loop surfaces an offer → fetch command pulls the EB body.
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(20);
        let eb = Point::new(7, vec![0xAB; 32]);

        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        drain_outputs(&mut behavior);
        complete_handshake_leios(&mut behavior, &pid);
        assert!(behavior.peers.get(&pid).unwrap().supports_leios());

        // Housekeeping drives the notify pull loop once Leios is negotiated.
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);
        assert!(
            outputs.has_send(|m| matches!(m, AnyMessage::LeiosNotify(ln::Message::RequestNext))),
            "should request next leios notification"
        );

        // Server moves to Busy (our request was sent), then offers an EB.
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::LeiosNotify(ln::Message::RequestNext),
        ));
        drain_outputs(&mut behavior);

        let offer = AnyMessage::LeiosNotify(ln::Message::BlockOffer(eb.clone(), 99));
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![offer]));
        let outputs = drain_outputs(&mut behavior);
        assert!(
            outputs.has_event(|e| matches!(e, InitiatorEvent::EbNotification(..))),
            "should surface the EB offer as an event"
        );

        // Application requests the EB body; housekeeping sends the fetch request.
        behavior.execute(InitiatorCommand::FetchEb(pid.clone(), eb.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);
        assert!(
            outputs.has_send(|m| matches!(m, AnyMessage::LeiosFetch(lf::Message::BlockRequest(_)))),
            "should send a leios-fetch block request"
        );

        // Server delivers the EB body, surfaced as EbFetched.
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::LeiosFetch(lf::Message::BlockRequest(eb.clone())),
        ));
        drain_outputs(&mut behavior);

        let block =
            AnyMessage::LeiosFetch(lf::Message::Block(AnyCbor::from_raw_bytes(vec![1, 2, 3])));
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![block]));
        let outputs = drain_outputs(&mut behavior);
        assert!(
            outputs.has_event(|e| matches!(e, InitiatorEvent::EbFetched(..))),
            "should surface the fetched EB body as an event"
        );
    }

    #[tokio::test]
    async fn fetch_command_sends_request_without_housekeeping() {
        // Composition: a fetch command enqueues and serves in one step, so the
        // request is sent without running the rest of housekeeping.
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(21);
        let eb = Point::new(7, vec![0xCD; 32]);

        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        drain_outputs(&mut behavior);
        complete_handshake_leios(&mut behavior, &pid);
        drain_outputs(&mut behavior);

        // Housekeeping drives the notify pull loop, so RequestNext marks a turn.
        behavior.execute(InitiatorCommand::Housekeeping);
        let ticked = drain_outputs(&mut behavior);
        assert!(
            ticked.has_send(|m| matches!(m, AnyMessage::LeiosNotify(ln::Message::RequestNext))),
            "should send RequestNext on a housekeeping tick"
        );

        // Issuing a fetch, with no tick of any kind.
        behavior.execute(InitiatorCommand::FetchEb(pid.clone(), eb.clone()));
        let issued = drain_outputs(&mut behavior);

        assert!(
            issued.has_send(|m| matches!(m, AnyMessage::LeiosFetch(lf::Message::BlockRequest(_)))),
            "should send the leios-fetch request when it is issued"
        );
        assert!(
            !issued.has_send(|m| matches!(m, AnyMessage::LeiosNotify(ln::Message::RequestNext))),
            "should NOT drive the notify pull loop"
        );
    }

    #[tokio::test]
    async fn eb_request_queued_before_the_handshake_is_sent_on_accept() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(22);
        let eb = Point::new(7, vec![0xEF; 32]);
        include_and_connect(&mut behavior, &pid);

        behavior.execute(InitiatorCommand::FetchEb(pid.clone(), eb.clone()));
        let issued = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(issued.len(), 0, "the fetch waits for the handshake");

        let accepted = complete_handshake_leios(&mut behavior, &pid);
        let sent = sends_to(&accepted, &pid, is_eb_request);
        assert_eq!(sent.len(), 1, "one fetch when the handshake completes");
        assert!(
            matches!(
                &sent[0],
                AnyMessage::LeiosFetch(lf::Message::BlockRequest(p)) if *p == eb
            ),
            "the queued fetch goes on the handshake, got {sent:?}"
        );
        assert!(!behavior.peers[&pid].violation, "no violation");
    }

    fn sends_to(
        outputs: &[BehaviorOutput<InitiatorBehavior>],
        pid: &PeerId,
        pred: fn(&AnyMessage) -> bool,
    ) -> Vec<AnyMessage> {
        outputs
            .sends()
            .filter(|&(p, m)| p == pid && pred(m))
            .map(|(_, m)| m.clone())
            .collect()
    }

    fn is_eb_request(m: &AnyMessage) -> bool {
        matches!(m, AnyMessage::LeiosFetch(lf::Message::BlockRequest(_)))
    }

    fn include_and_connect(behavior: &mut InitiatorBehavior, pid: &PeerId) {
        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(behavior);
        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        drain_outputs(behavior);
    }

    fn range(n: u64) -> (Point, Point) {
        (
            Point::new(n, vec![n as u8; 32]),
            Point::new(n + 1, vec![n as u8 + 1; 32]),
        )
    }

    fn is_range(m: &AnyMessage, want: &(Point, Point)) -> bool {
        matches!(m, AnyMessage::BlockFetch(bf::Message::RequestRange(x)) if x == want)
    }

    fn is_any_range(m: &AnyMessage) -> bool {
        matches!(m, AnyMessage::BlockFetch(bf::Message::RequestRange(_)))
    }

    fn is_keepalive(m: &AnyMessage) -> bool {
        matches!(m, AnyMessage::KeepAlive(keepalive::Message::KeepAlive(_)))
    }

    fn is_notify_next(m: &AnyMessage) -> bool {
        matches!(m, AnyMessage::LeiosNotify(ln::Message::RequestNext))
    }

    fn housekeep_twice(behavior: &mut InitiatorBehavior) -> Vec<BehaviorOutput<InitiatorBehavior>> {
        behavior.execute(InitiatorCommand::Housekeeping);
        let mut outputs = drain_outputs(behavior);
        behavior.execute(InitiatorCommand::Housekeeping);
        outputs.extend(drain_outputs(behavior));
        outputs
    }

    #[tokio::test]
    async fn blockfetch_requests_once_until_sent() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(40);
        behavior.blockfetch.enqueue(range(1));
        behavior.blockfetch.enqueue(range(3));
        include_and_connect(&mut behavior, &pid);
        let mut outputs = complete_handshake(&mut behavior, &pid);
        outputs.extend(housekeep_twice(&mut behavior));
        assert_eq!(
            outputs.sends().filter(|&(_, m)| is_any_range(m)).count(),
            1,
            "RequestRange count over the handshake and two passes"
        );
    }

    #[tokio::test]
    async fn leiosfetch_requests_once_until_sent() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(41);
        include_and_connect(&mut behavior, &pid);
        complete_handshake_leios(&mut behavior, &pid);
        behavior
            .leiosfetch
            .enqueue(pid.clone(), FetchRequest::Block(Point::new(7, vec![7; 32])));
        behavior
            .leiosfetch
            .enqueue(pid.clone(), FetchRequest::Block(Point::new(8, vec![8; 32])));
        let outputs = housekeep_twice(&mut behavior);
        let n = outputs
            .sends()
            .filter(|&(_, m)| matches!(m, AnyMessage::LeiosFetch(lf::Message::BlockRequest(_))))
            .count();
        assert_eq!(n, 1, "BlockRequest count over two passes");
    }

    #[tokio::test]
    async fn chainsync_finds_intersect_once_until_sent() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(42);
        behavior.execute(InitiatorCommand::StartSync(vec![Point::Origin]));
        include_and_connect(&mut behavior, &pid);
        complete_handshake(&mut behavior, &pid);
        let outputs = housekeep_twice(&mut behavior);
        let n = outputs
            .sends()
            .filter(|&(_, m)| matches!(m, AnyMessage::ChainSync(cs::Message::FindIntersect(_))))
            .count();
        assert_eq!(n, 1, "FindIntersect count over two passes");
    }

    #[tokio::test]
    async fn keepalive_requests_once_until_sent() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(43);
        include_and_connect(&mut behavior, &pid);
        complete_handshake(&mut behavior, &pid);
        let outputs = housekeep_twice(&mut behavior);
        assert_eq!(
            outputs.sends().filter(|&(_, m)| is_keepalive(m)).count(),
            1,
            "KeepAlive count over two passes"
        );
    }

    #[tokio::test]
    async fn leiosnotify_requests_once_until_sent() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(44);
        include_and_connect(&mut behavior, &pid);
        complete_handshake_leios(&mut behavior, &pid);
        let outputs = housekeep_twice(&mut behavior);
        assert_eq!(
            outputs.sends().filter(|&(_, m)| is_notify_next(m)).count(),
            1,
            "leios RequestNext count over two passes"
        );
    }

    #[tokio::test]
    async fn peersharing_requests_once_until_sent() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(45);
        include_and_connect(&mut behavior, &pid);
        complete_handshake(&mut behavior, &pid);
        assert!(
            behavior.peers.get(&pid).unwrap().supports_peer_sharing(),
            "setup: peer sharing negotiated"
        );
        let outputs = housekeep_twice(&mut behavior);
        let n = outputs
            .sends()
            .filter(|&(_, m)| {
                matches!(
                    m,
                    AnyMessage::PeerSharing(peersharing::Message::ShareRequest(_))
                )
            })
            .count();
        assert_eq!(n, 1, "ShareRequest count over two passes");
    }

    fn push_first_range_to(behavior: &mut InitiatorBehavior, pid: &PeerId, other: &PeerId) {
        include_and_connect(behavior, pid);
        include_and_connect(behavior, other);
        complete_handshake(behavior, pid);
        behavior.blockfetch.enqueue(range(1));
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(behavior);
        assert_eq!(
            outputs
                .sends()
                .filter(|&(p, m)| p == pid && is_range(m, &range(1)))
                .count(),
            1,
            "setup: range 1 pushed to pid"
        );
    }

    fn ranges_requested_from(behavior: &mut InitiatorBehavior, peer: &PeerId) -> Vec<AnyMessage> {
        let mut outputs = complete_handshake(behavior, peer);
        behavior.execute(InitiatorCommand::Housekeeping);
        outputs.extend(drain_outputs(behavior));
        sends_to(&outputs, peer, is_any_range)
    }

    #[tokio::test]
    async fn other_range_sent_first_keeps_unsent_range() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let (pid, other) = (PeerId::test(46), PeerId::test(47));
        push_first_range_to(&mut behavior, &pid, &other);
        behavior
            .blockfetch
            .request_block_batch(&pid, range(5), &mut behavior.outbound);
        let outputs = drain_outputs(&mut behavior);
        assert_eq!(
            outputs
                .sends()
                .filter(|&(_, m)| is_range(m, &range(5)))
                .count(),
            1,
            "setup: range 5 pushed by the caller"
        );
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::BlockFetch(bf::Message::RequestRange(range(5))),
        ));
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::BlockFetch(bf::Message::RequestRange(range(1))),
        ));
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);
        assert_eq!(
            outputs
                .sends()
                .filter(|&(p, m)| *p == pid && is_any_range(m))
                .count(),
            0,
            "no second range pushed to pid"
        );
        behavior.handle_io(InterfaceEvent::Disconnected(pid.clone()));
        drain_outputs(&mut behavior);
        let requested = ranges_requested_from(&mut behavior, &other);
        assert!(
            requested
                .first()
                .map(|m| is_range(m, &range(1)))
                .unwrap_or(false),
            "range 1 at queue front, other got {requested:?}"
        );
    }

    #[tokio::test]
    async fn disconnect_before_sent_requeues_range() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let (pid, other) = (PeerId::test(50), PeerId::test(51));
        push_first_range_to(&mut behavior, &pid, &other);
        behavior.handle_io(InterfaceEvent::Disconnected(pid.clone()));
        drain_outputs(&mut behavior);
        let requested = ranges_requested_from(&mut behavior, &other);
        assert!(
            requested
                .first()
                .map(|m| is_range(m, &range(1)))
                .unwrap_or(false),
            "range 1 requeued, other got {requested:?}"
        );
    }

    #[tokio::test]
    async fn range_requests_go_one_to_each_idle_peer() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let (a, b) = (PeerId::test(38), PeerId::test(39));
        include_and_connect(&mut behavior, &a);
        include_and_connect(&mut behavior, &b);
        complete_handshake(&mut behavior, &a);
        complete_handshake(&mut behavior, &b);

        behavior.execute(InitiatorCommand::RequestBlocks(range(1)));
        behavior.execute(InitiatorCommand::RequestBlocks(range(3)));
        let outputs = drain_outputs(&mut behavior);
        assert_eq!(
            (
                sends_to(&outputs, &a, is_any_range).len(),
                sends_to(&outputs, &b, is_any_range).len()
            ),
            (1, 1),
            "RequestRange counts to a and b"
        );
    }

    #[tokio::test]
    async fn held_range_request_is_sent_once_the_first_is_answered() {
        tokio::time::pause();
        let no_blocks = vec![AnyMessage::BlockFetch(bf::Message::NoBlocks)];
        for reply in [no_blocks, full_batch()] {
            let mut behavior = InitiatorBehavior::default();
            let pid = PeerId::test(43);
            send_first_of_two_ranges(&mut behavior, &pid);

            let (last, streaming) = reply.split_last().expect("a reply has a message");
            behavior.handle_io(InterfaceEvent::Recv(pid.clone(), streaming.to_vec()));
            let during = sends_to(&drain_outputs(&mut behavior), &pid, is_any_range);
            assert_eq!(
                during.len(),
                0,
                "the held range waits while the reply streams, reply={reply:?}"
            );

            behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![last.clone()]));
            let next = sends_to(&drain_outputs(&mut behavior), &pid, is_any_range);
            assert_eq!(
                next.len(),
                1,
                "one RequestRange once the first is answered, reply={reply:?}"
            );
            assert!(
                is_range(&next[0], &range(3)),
                "the held range goes once the first is answered, reply={reply:?}, got {next:?}"
            );
            assert!(
                !behavior.peers[&pid].violation,
                "no violation, reply={reply:?}"
            );
        }
    }

    fn full_batch() -> Vec<AnyMessage> {
        vec![
            AnyMessage::BlockFetch(bf::Message::StartBatch),
            AnyMessage::BlockFetch(bf::Message::Block(vec![0xBE; 8])),
            AnyMessage::BlockFetch(bf::Message::BatchDone),
        ]
    }

    fn send_first_of_two_ranges(behavior: &mut InitiatorBehavior, pid: &PeerId) {
        include_and_connect(behavior, pid);
        complete_handshake(behavior, pid);
        behavior.execute(InitiatorCommand::RequestBlocks(range(1)));
        behavior.execute(InitiatorCommand::RequestBlocks(range(3)));
        let first = sends_to(&drain_outputs(behavior), pid, is_any_range);
        assert_eq!(
            first.len(),
            1,
            "setup: one RequestRange for the first range"
        );
        behavior.execute(InitiatorCommand::Housekeeping);
        let waiting = sends_to(&drain_outputs(behavior), pid, is_any_range);
        assert_eq!(
            waiting.len(),
            0,
            "setup: the held range waits for the first"
        );
        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), first[0].clone()));
    }

    fn finish_batch_then_disconnect(
        behavior: &mut InitiatorBehavior,
        pid: &PeerId,
        other: &PeerId,
        dropping: Option<InitiatorCommand>,
    ) -> (usize, Vec<AnyMessage>) {
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), full_batch()));
        let mut to_pid = sends_to(&drain_outputs(behavior), pid, is_any_range);
        // The interface writes a range pushed while the peer is still connected.
        for m in &to_pid {
            behavior.handle_io(InterfaceEvent::Sent(pid.clone(), m.clone()));
        }
        if let Some(command) = dropping {
            behavior.execute(command);
            drain_outputs(behavior);
        }

        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(behavior);
        assert!(outputs.has_disconnect_for(pid), "setup: pid disconnected");
        to_pid.extend(sends_to(&outputs, pid, is_any_range));
        behavior.handle_io(InterfaceEvent::Disconnected(pid.clone()));
        drain_outputs(behavior);

        include_and_connect(behavior, other);
        (to_pid.len(), ranges_requested_from(behavior, other))
    }

    #[tokio::test]
    async fn range_freed_by_a_dropped_peer_goes_to_another_peer() {
        tokio::time::pause();
        for dropped in [PromotionTag::Banned, PromotionTag::Cold] {
            let mut behavior = InitiatorBehavior::default();
            let (pid, other) = (PeerId::test(54), PeerId::test(55));
            send_first_of_two_ranges(&mut behavior, &pid);

            if dropped == PromotionTag::Banned {
                let bad = AnyMessage::KeepAlive(keepalive::Message::ResponseKeepAlive(7));
                behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![bad]));
            } else {
                behavior.execute(InitiatorCommand::DemotePeer(pid.clone()));
            }
            drain_outputs(&mut behavior);
            assert_eq!(
                behavior.peers[&pid].promotion, dropped,
                "setup: pid dropped"
            );

            let (to_pid, requested) =
                finish_batch_then_disconnect(&mut behavior, &pid, &other, None);
            assert_eq!(
                (to_pid, requested.iter().any(|m| is_range(m, &range(3)))),
                (0, true),
                "ranges sent to the {dropped:?} peer, and range 3 sent to the other peer"
            );
        }
    }

    #[tokio::test]
    async fn range_written_to_a_peer_dropped_after_its_batch_goes_to_another_peer() {
        tokio::time::pause();
        let (pid, other) = (PeerId::test(66), PeerId::test(67));
        for dropping in [
            InitiatorCommand::BanPeer(pid.clone()),
            InitiatorCommand::DemotePeer(pid.clone()),
        ] {
            let label = format!("{dropping:?}");
            let mut behavior = InitiatorBehavior::default();
            send_first_of_two_ranges(&mut behavior, &pid);

            let (to_pid, requested) =
                finish_batch_then_disconnect(&mut behavior, &pid, &other, Some(dropping));
            assert_eq!(
                (
                    to_pid,
                    requested.first().is_some_and(|m| is_range(m, &range(3)))
                ),
                (1, true),
                "ranges sent to pid, and range 3 first to the other peer, after {label}"
            );
        }
    }

    #[tokio::test]
    async fn range_unanswered_at_disconnect_goes_to_another_peer() {
        tokio::time::pause();
        for received in [vec![], full_batch()[..2].to_vec()] {
            let mut behavior = InitiatorBehavior::default();
            let (pid, other) = (PeerId::test(70), PeerId::test(71));
            send_first_of_two_ranges(&mut behavior, &pid);
            behavior.handle_io(InterfaceEvent::Recv(pid.clone(), received.clone()));
            drain_outputs(&mut behavior);

            behavior.handle_io(InterfaceEvent::Disconnected(pid.clone()));
            drain_outputs(&mut behavior);
            include_and_connect(&mut behavior, &other);
            let requested = ranges_requested_from(&mut behavior, &other);
            assert!(
                requested.first().is_some_and(|m| is_range(m, &range(1))),
                "range 1 at queue front, received={received:?}, other got {requested:?}"
            );
        }
    }

    #[tokio::test]
    async fn range_of_a_finished_batch_is_not_requested_again() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let (pid, other) = (PeerId::test(74), PeerId::test(75));
        include_and_connect(&mut behavior, &pid);
        complete_handshake(&mut behavior, &pid);
        behavior.execute(InitiatorCommand::RequestBlocks(range(1)));
        let first = sends_to(&drain_outputs(&mut behavior), &pid, is_any_range);
        assert_eq!(first.len(), 1, "setup: range 1 pushed to pid");
        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), first[0].clone()));
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), full_batch()));
        drain_outputs(&mut behavior);

        behavior.handle_io(InterfaceEvent::Disconnected(pid.clone()));
        drain_outputs(&mut behavior);
        behavior.execute(InitiatorCommand::RequestBlocks(range(3)));
        drain_outputs(&mut behavior);
        include_and_connect(&mut behavior, &other);
        let requested = ranges_requested_from(&mut behavior, &other);
        assert!(
            requested.first().is_some_and(|m| is_range(m, &range(3))),
            "range 3 at queue front, other got {requested:?}"
        );
    }

    #[tokio::test]
    async fn range_requeued_at_disconnect_goes_to_one_peer() {
        tokio::time::pause();
        for written in [false, true] {
            let mut behavior = InitiatorBehavior::default();
            let (pid, b, c) = (PeerId::test(76), PeerId::test(77), PeerId::test(78));
            include_and_connect(&mut behavior, &pid);
            complete_handshake(&mut behavior, &pid);
            behavior.execute(InitiatorCommand::RequestBlocks(range(1)));
            let first = sends_to(&drain_outputs(&mut behavior), &pid, is_any_range);
            assert_eq!(first.len(), 1, "setup: range 1 pushed to pid");
            if written {
                behavior.handle_io(InterfaceEvent::Sent(pid.clone(), first[0].clone()));
            }

            behavior.handle_io(InterfaceEvent::Disconnected(pid.clone()));
            drain_outputs(&mut behavior);
            include_and_connect(&mut behavior, &b);
            include_and_connect(&mut behavior, &c);
            let mut outputs = complete_handshake(&mut behavior, &b);
            outputs.extend(complete_handshake(&mut behavior, &c));
            behavior.execute(InitiatorCommand::Housekeeping);
            outputs.extend(drain_outputs(&mut behavior));
            assert_eq!(
                outputs
                    .sends()
                    .filter(|&(_, m)| is_range(m, &range(1)))
                    .count(),
                1,
                "requests for range 1 after the disconnect, written={written}"
            );
        }
    }

    #[tokio::test]
    async fn requested_ranges_skip_a_banned_peer() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let (pid, other) = (PeerId::test(58), PeerId::test(59));
        include_and_connect(&mut behavior, &pid);
        include_and_connect(&mut behavior, &other);
        complete_handshake(&mut behavior, &pid);
        complete_handshake(&mut behavior, &other);

        behavior.execute(InitiatorCommand::BanPeer(pid.clone()));
        drain_outputs(&mut behavior);
        behavior.execute(InitiatorCommand::RequestBlocks(range(1)));
        behavior.execute(InitiatorCommand::RequestBlocks(range(3)));
        let outputs = drain_outputs(&mut behavior);
        assert_eq!(
            (
                sends_to(&outputs, &pid, is_any_range).len(),
                sends_to(&outputs, &other, is_any_range)
                    .iter()
                    .any(|m| is_range(m, &range(1)))
            ),
            (0, true),
            "ranges sent to the banned peer, and range 1 sent to the other peer"
        );
    }

    fn is_sync_next(m: &AnyMessage) -> bool {
        matches!(m, AnyMessage::ChainSync(cs::Message::RequestNext))
    }

    fn start_sync_and_intersect(behavior: &mut InitiatorBehavior, pid: &PeerId) -> cs::Tip {
        behavior.execute(InitiatorCommand::StartSync(vec![Point::Origin]));
        include_and_connect(behavior, pid);
        handshake_and_intersect(behavior, pid)
    }

    fn handshake_and_intersect(behavior: &mut InitiatorBehavior, pid: &PeerId) -> cs::Tip {
        complete_handshake(behavior, pid);
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(behavior);
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::ChainSync(cs::Message::FindIntersect(vec![Point::Origin])),
        ));
        let tip = cs::Tip(Point::new(9, vec![9; 32]), 9);
        behavior.handle_io(InterfaceEvent::Recv(
            pid.clone(),
            vec![AnyMessage::ChainSync(cs::Message::IntersectFound(
                Point::Origin,
                tip.clone(),
            ))],
        ));
        drain_outputs(behavior);
        tip
    }

    fn continue_sync_n_times(behavior: &mut InitiatorBehavior, pid: &PeerId, n: usize) -> usize {
        let mut sent = 0;
        for _ in 0..n {
            behavior.execute(InitiatorCommand::ContinueSync(pid.clone()));
            sent += drain_outputs(behavior)
                .sends()
                .filter(|&(_, m)| is_sync_next(m))
                .count();
        }
        sent
    }

    fn answer_sync_next(behavior: &mut InitiatorBehavior, pid: &PeerId, tip: &cs::Tip) -> usize {
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::ChainSync(cs::Message::RequestNext),
        ));
        drain_outputs(behavior);
        let hdr = cs::HeaderContent {
            variant: 1,
            byron_prefix: None,
            cbor: vec![0xBE; 32],
        };
        behavior.handle_io(InterfaceEvent::Recv(
            pid.clone(),
            vec![AnyMessage::ChainSync(cs::Message::RollForward(
                hdr,
                tip.clone(),
            ))],
        ));
        let outputs = drain_outputs(behavior);
        outputs.sends().filter(|&(_, m)| is_sync_next(m)).count()
    }

    #[tokio::test]
    async fn continue_sync_before_sent_defers_next() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(52);
        let tip = start_sync_and_intersect(&mut behavior, &pid);
        let counts = [
            continue_sync_n_times(&mut behavior, &pid, 1),
            answer_sync_next(&mut behavior, &pid, &tip),
            continue_sync_n_times(&mut behavior, &pid, 2),
            answer_sync_next(&mut behavior, &pid, &tip),
            answer_sync_next(&mut behavior, &pid, &tip),
        ];
        assert_eq!(
            counts,
            [1, 0, 1, 1, 0],
            "RequestNext counts: one ContinueSync and its answer, two ContinueSync and two answers"
        );
    }

    #[tokio::test]
    async fn inbound_before_sent_keeps_unsent_keepalive() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(63);
        start_sync_and_intersect(&mut behavior, &pid);
        behavior.execute(InitiatorCommand::Housekeeping);
        let before = drain_outputs(&mut behavior)
            .sends()
            .filter(|&(_, m)| is_keepalive(m))
            .count();
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::KeepAlive(keepalive::Message::KeepAlive(u16::MAX)),
        ));
        behavior.handle_io(InterfaceEvent::Recv(
            pid.clone(),
            vec![AnyMessage::KeepAlive(
                keepalive::Message::ResponseKeepAlive(u16::MAX),
            )],
        ));
        drain_outputs(&mut behavior);
        behavior.execute(InitiatorCommand::Housekeeping);
        let after = drain_outputs(&mut behavior)
            .sends()
            .filter(|&(_, m)| is_keepalive(m))
            .count();
        assert_eq!(
            (before, after),
            (0, 1),
            "KeepAlive counts after an inbound IntersectFound, then after the KeepAlive is answered"
        );
    }

    #[tokio::test]
    async fn reconnect_drops_deferred_next() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(64);
        start_sync_and_intersect(&mut behavior, &pid);
        assert_eq!(
            continue_sync_n_times(&mut behavior, &pid, 2),
            1,
            "setup: one RequestNext unsent and one deferred"
        );
        behavior.handle_io(InterfaceEvent::Disconnected(pid.clone()));
        drain_outputs(&mut behavior);
        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        drain_outputs(&mut behavior);
        handshake_and_intersect(&mut behavior, &pid);
        assert_eq!(
            continue_sync_n_times(&mut behavior, &pid, 1),
            1,
            "RequestNext for the first ContinueSync after reconnect"
        );
    }

    #[tokio::test]
    async fn inbound_before_sent_keeps_deferred_next() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(65);
        let tip = start_sync_and_intersect(&mut behavior, &pid);
        let first = continue_sync_n_times(&mut behavior, &pid, 2);
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::KeepAlive(keepalive::Message::KeepAlive(u16::MAX)),
        ));
        behavior.handle_io(InterfaceEvent::Recv(
            pid.clone(),
            vec![AnyMessage::KeepAlive(
                keepalive::Message::ResponseKeepAlive(u16::MAX),
            )],
        ));
        let during = drain_outputs(&mut behavior)
            .sends()
            .filter(|&(_, m)| is_sync_next(m))
            .count();
        let counts = [first, during, answer_sync_next(&mut behavior, &pid, &tip)];
        assert_eq!(
            counts,
            [1, 0, 1],
            "RequestNext counts: two ContinueSync, an inbound KeepAlive response, then the first answer"
        );
    }

    #[tokio::test]
    async fn reconnect_after_disconnect_sends_again() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(56);
        include_and_connect(&mut behavior, &pid);
        complete_handshake(&mut behavior, &pid);
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);
        behavior.handle_io(InterfaceEvent::Disconnected(pid.clone()));
        drain_outputs(&mut behavior);
        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        drain_outputs(&mut behavior);
        complete_handshake(&mut behavior, &pid);
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);
        assert_eq!(
            outputs.sends().filter(|&(_, m)| is_keepalive(m)).count(),
            1,
            "next session sends KeepAlive"
        );
    }

    #[tokio::test]
    async fn keepalive_sent_and_answered_sends_again() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(58);
        include_and_connect(&mut behavior, &pid);
        complete_handshake(&mut behavior, &pid);
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::KeepAlive(keepalive::Message::KeepAlive(u16::MAX)),
        ));
        behavior.handle_io(InterfaceEvent::Recv(
            pid.clone(),
            vec![AnyMessage::KeepAlive(
                keepalive::Message::ResponseKeepAlive(u16::MAX),
            )],
        ));
        drain_outputs(&mut behavior);
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);
        assert_eq!(
            outputs.sends().filter(|&(_, m)| is_keepalive(m)).count(),
            1,
            "KeepAlive after the first is sent and answered"
        );
    }

    #[tokio::test]
    async fn keepalive_sent_and_unanswered_sends_nothing() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(62);
        include_and_connect(&mut behavior, &pid);
        complete_handshake(&mut behavior, &pid);
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::KeepAlive(keepalive::Message::KeepAlive(u16::MAX)),
        ));
        drain_outputs(&mut behavior);
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);
        assert_eq!(
            outputs.sends().filter(|&(_, m)| is_keepalive(m)).count(),
            0,
            "KeepAlive while the first awaits its response"
        );
    }

    #[test]
    fn try_send_request_refuses_keepalive_response() {
        let pid = PeerId::test(61);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::<InitiatorBehavior>::new();
        state.keepalive = keepalive::State::Server(7);
        let response = AnyMessage::KeepAlive(keepalive::Message::ResponseKeepAlive(7));
        let pushed = state.try_send_request(&pid, response, &mut outbound);
        assert_eq!(
            (pushed, outbound.drain_ready().len()),
            (false, 0),
            "pushed and sends for a ResponseKeepAlive"
        );
        state.keepalive = keepalive::State::default();
        let request = AnyMessage::KeepAlive(keepalive::Message::KeepAlive(7));
        assert!(
            state.try_send_request(&pid, request, &mut outbound),
            "no unsent keepalive after the response"
        );
    }

    #[tokio::test]
    async fn leiosnotify_sent_and_answered_sends_again() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(59);
        include_and_connect(&mut behavior, &pid);
        complete_handshake_leios(&mut behavior, &pid);
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::LeiosNotify(ln::Message::RequestNext),
        ));
        let offer = ln::Message::BlockOffer(Point::new(7, vec![7; 32]), 9);
        behavior.handle_io(InterfaceEvent::Recv(
            pid.clone(),
            vec![AnyMessage::LeiosNotify(offer)],
        ));
        drain_outputs(&mut behavior);
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);
        assert_eq!(
            outputs.sends().filter(|&(_, m)| is_notify_next(m)).count(),
            1,
            "leios RequestNext after the first is sent and answered"
        );
    }

    #[tokio::test]
    async fn leiosfetch_sent_and_answered_sends_next() {
        tokio::time::pause();
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(60);
        include_and_connect(&mut behavior, &pid);
        complete_handshake_leios(&mut behavior, &pid);
        let (first, second) = (Point::new(7, vec![7; 32]), Point::new(8, vec![8; 32]));
        behavior
            .leiosfetch
            .enqueue(pid.clone(), FetchRequest::Block(first.clone()));
        behavior
            .leiosfetch
            .enqueue(pid.clone(), FetchRequest::Block(second.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);
        behavior.handle_io(InterfaceEvent::Sent(
            pid.clone(),
            AnyMessage::LeiosFetch(lf::Message::BlockRequest(first)),
        ));
        let waiting = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(waiting.len(), 0, "the queued fetch waits for the reply");

        let body = lf::Message::Block(AnyCbor::from_raw_bytes(vec![1]));
        behavior.handle_io(InterfaceEvent::Recv(
            pid.clone(),
            vec![AnyMessage::LeiosFetch(body)],
        ));
        let next = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(
            next.len(),
            1,
            "second BlockRequest when the first is answered"
        );
        assert!(
            matches!(
                &next[0],
                AnyMessage::LeiosFetch(lf::Message::BlockRequest(p)) if *p == second
            ),
            "the second fetch goes on the reply, got {next:?}"
        );
        assert!(!behavior.peers[&pid].violation, "no violation");
    }
}
