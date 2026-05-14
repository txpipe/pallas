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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        MAINNET_MAGIC, Point, blockfetch as bf, chainsync as cs, handshake, keepalive, peersharing,
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

    fn complete_handshake(behavior: &mut InitiatorBehavior, pid: &PeerId) {
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
        drain_outputs(behavior);
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

        // Complete handshake → Initialized
        complete_handshake(&mut behavior, &pid);

        // Re-enqueue since housekeeping may have consumed nothing
        // (the request is still in the queue since peer wasn't available)
        // Housekeeping now — peer is Initialized + blockfetch Idle
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);
        assert!(
            outputs.has_send(|m| matches!(m, AnyMessage::BlockFetch(bf::Message::RequestRange(_)))),
            "should send RequestRange after handshake completes"
        );
    }
}
