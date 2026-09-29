use futures::{Stream, StreamExt, stream::FusedStream};
use std::{collections::HashMap, task::Poll};

use crate::{
    Behavior, BehaviorOutput, InterfaceCommand, Message as MessageTrait, OutboundQueue, PeerId,
    protocol as proto,
};

use super::{AcceptedVersion, AnyMessage, BlockRange, ConnectionState};

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
        self.violation = false;
    }
}

/// Applies `msg` to `state` and queues it as a send to `pid`.
fn send_to_peer(
    pid: &PeerId,
    state: &mut InitiatorState,
    msg: AnyMessage,
    outbound: &mut OutboundQueue<InitiatorBehavior>,
) {
    state.apply_msg(&msg);
    outbound.push_ready(InterfaceCommand::Send(pid.clone(), msg));
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

    /// Applies a message the caller sent itself and notifies visitors.
    #[deprecated(
        since = "1.5.0",
        note = "pass `InterfaceEvent::Sent` to `handle_io`, and call `InitiatorState::apply_msg` before queueing a message of your own"
    )]
    pub fn on_outbound_msg(&mut self, pid: &PeerId, msg: &AnyMessage) {
        if let Some(state) = self.peers.get_mut(pid) {
            state.apply_msg(msg);
        }
        self.on_sent(pid, msg);
    }

    #[tracing::instrument(skip_all, fields(pid = %pid, channel = %msg.channel()))]
    fn on_sent(&mut self, pid: &PeerId, msg: &AnyMessage) {
        tracing::debug!(channel = msg.channel(), "new outbound message");

        self.peers.entry(pid.clone()).and_modify(|state| {
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
                self.on_sent(pid, msg);
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

    /// Completes a handshake negotiating a Leios-capable version (15).
    fn complete_handshake_leios(behavior: &mut InitiatorBehavior, pid: &PeerId) {
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

        // Confirm the ShareRequest that housekeeping sent.
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

        // Confirm the RequestNext, then the server offers an EB.
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

    fn connect_peer(behavior: &mut InitiatorBehavior, pid: &PeerId, leios: bool) {
        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(behavior);

        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        drain_outputs(behavior);

        if leios {
            complete_handshake_leios(behavior, pid);
        } else {
            complete_handshake(behavior, pid);
        }
    }

    fn sends_to(
        outputs: &[BehaviorOutput<InitiatorBehavior>],
        pid: &PeerId,
        pred: impl Fn(&AnyMessage) -> bool,
    ) -> Vec<AnyMessage> {
        outputs
            .iter()
            .filter_map(|o| match o {
                BehaviorOutput::InterfaceCommand(crate::InterfaceCommand::Send(p, m))
                    if p == pid && pred(m) =>
                {
                    Some(m.clone())
                }
                _ => None,
            })
            .collect()
    }

    fn is_keepalive(m: &AnyMessage) -> bool {
        matches!(m, AnyMessage::KeepAlive(keepalive::Message::KeepAlive(_)))
    }

    fn is_share_request(m: &AnyMessage) -> bool {
        matches!(
            m,
            AnyMessage::PeerSharing(peersharing::Message::ShareRequest(_))
        )
    }

    fn is_notify_request(m: &AnyMessage) -> bool {
        matches!(m, AnyMessage::LeiosNotify(ln::Message::RequestNext))
    }

    fn is_find_intersect(m: &AnyMessage) -> bool {
        matches!(m, AnyMessage::ChainSync(cs::Message::FindIntersect(_)))
    }

    fn is_sync_request(m: &AnyMessage) -> bool {
        matches!(m, AnyMessage::ChainSync(cs::Message::RequestNext))
    }

    fn is_range_request(m: &AnyMessage) -> bool {
        matches!(m, AnyMessage::BlockFetch(bf::Message::RequestRange(_)))
    }

    fn is_eb_request(m: &AnyMessage) -> bool {
        matches!(m, AnyMessage::LeiosFetch(lf::Message::BlockRequest(_)))
    }

    #[tokio::test]
    async fn keepalive_is_sent_again_once_answered() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(30);
        connect_peer(&mut behavior, &pid, false);

        behavior.execute(InitiatorCommand::Housekeeping);
        let first = sends_to(&drain_outputs(&mut behavior), &pid, is_keepalive);
        assert_eq!(first.len(), 1, "first pass sends one keepalive");

        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), first[0].clone()));
        let AnyMessage::KeepAlive(keepalive::Message::KeepAlive(cookie)) = first[0] else {
            unreachable!()
        };
        let reply = AnyMessage::KeepAlive(keepalive::Message::ResponseKeepAlive(cookie));
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![reply]));
        drain_outputs(&mut behavior);

        behavior.execute(InitiatorCommand::Housekeeping);
        let second = sends_to(&drain_outputs(&mut behavior), &pid, is_keepalive);
        assert_eq!(second.len(), 1, "an answered keepalive is sent again");
    }

    #[tokio::test]
    async fn keepalive_is_not_sent_twice_before_sent() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(31);
        connect_peer(&mut behavior, &pid, false);

        behavior.execute(InitiatorCommand::Housekeeping);
        let first = sends_to(&drain_outputs(&mut behavior), &pid, is_keepalive);
        assert_eq!(first.len(), 1, "first pass sends one keepalive");

        behavior.execute(InitiatorCommand::Housekeeping);
        let second = sends_to(&drain_outputs(&mut behavior), &pid, is_keepalive);
        assert_eq!(
            second.len(),
            0,
            "second pass before Sent sends no keepalive"
        );
    }

    #[tokio::test]
    async fn share_request_is_not_sent_twice_before_sent() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(32);
        connect_peer(&mut behavior, &pid, false);

        behavior.execute(InitiatorCommand::Housekeeping);
        let first = sends_to(&drain_outputs(&mut behavior), &pid, is_share_request);
        assert_eq!(first.len(), 1, "first pass sends one share request");

        behavior.execute(InitiatorCommand::Housekeeping);
        let second = sends_to(&drain_outputs(&mut behavior), &pid, is_share_request);
        assert_eq!(
            second.len(),
            0,
            "second pass before Sent sends no share request"
        );
    }

    #[tokio::test]
    async fn notify_request_is_sent_again_once_answered() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(33);
        connect_peer(&mut behavior, &pid, true);

        behavior.execute(InitiatorCommand::Housekeeping);
        let first = sends_to(&drain_outputs(&mut behavior), &pid, is_notify_request);
        assert_eq!(first.len(), 1, "first pass sends one notify request");

        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), first[0].clone()));
        let offer = AnyMessage::LeiosNotify(ln::Message::BlockOffer(Point::new(7, vec![1; 32]), 9));
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![offer]));
        drain_outputs(&mut behavior);

        behavior.execute(InitiatorCommand::Housekeeping);
        let second = sends_to(&drain_outputs(&mut behavior), &pid, is_notify_request);
        assert_eq!(second.len(), 1, "an answered notify request is sent again");
    }

    #[tokio::test]
    async fn notify_request_is_not_sent_twice_before_sent() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(34);
        connect_peer(&mut behavior, &pid, true);

        behavior.execute(InitiatorCommand::Housekeeping);
        let first = sends_to(&drain_outputs(&mut behavior), &pid, is_notify_request);
        assert_eq!(first.len(), 1, "first pass sends one notify request");

        behavior.execute(InitiatorCommand::Housekeeping);
        let second = sends_to(&drain_outputs(&mut behavior), &pid, is_notify_request);
        assert_eq!(
            second.len(),
            0,
            "second pass before Sent sends no notify request"
        );
    }

    #[tokio::test]
    async fn find_intersect_is_not_sent_twice_before_sent() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(35);
        behavior.execute(InitiatorCommand::StartSync(vec![Point::Origin]));
        connect_peer(&mut behavior, &pid, false);

        behavior.execute(InitiatorCommand::Housekeeping);
        let first = sends_to(&drain_outputs(&mut behavior), &pid, is_find_intersect);
        assert_eq!(first.len(), 1, "first pass sends one FindIntersect");

        behavior.execute(InitiatorCommand::Housekeeping);
        let second = sends_to(&drain_outputs(&mut behavior), &pid, is_find_intersect);
        assert_eq!(
            second.len(),
            0,
            "second pass before Sent sends no FindIntersect"
        );
    }

    /// Brings `pid` to an idle chain-sync with an intersection found.
    fn intersect_peer(behavior: &mut InitiatorBehavior, pid: &PeerId) {
        behavior.execute(InitiatorCommand::StartSync(vec![Point::Origin]));
        connect_peer(behavior, pid, false);

        behavior.execute(InitiatorCommand::Housekeeping);
        let find = sends_to(&drain_outputs(behavior), pid, is_find_intersect);
        assert_eq!(find.len(), 1, "housekeeping sends one FindIntersect");

        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), find[0].clone()));
        let tip = cs::Tip(Point::Origin, 0);
        let found = AnyMessage::ChainSync(cs::Message::IntersectFound(Point::Origin, tip));
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![found]));
        drain_outputs(behavior);
    }

    #[tokio::test]
    async fn continue_sync_sends_after_intersection() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(36);
        intersect_peer(&mut behavior, &pid);

        behavior.execute(InitiatorCommand::ContinueSync(pid.clone()));
        let sent = sends_to(&drain_outputs(&mut behavior), &pid, is_sync_request);
        assert_eq!(sent.len(), 1, "ContinueSync sends one RequestNext");
    }

    #[tokio::test]
    async fn continue_sync_twice_before_sent_sends_once() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(37);
        intersect_peer(&mut behavior, &pid);

        behavior.execute(InitiatorCommand::ContinueSync(pid.clone()));
        behavior.execute(InitiatorCommand::ContinueSync(pid.clone()));
        let sent = sends_to(&drain_outputs(&mut behavior), &pid, is_sync_request);
        assert_eq!(
            sent.len(),
            1,
            "two ContinueSync before Sent send one RequestNext"
        );
    }

    #[tokio::test]
    async fn range_requests_go_one_to_each_idle_peer() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let a = PeerId::test(38);
        let b = PeerId::test(39);
        connect_peer(&mut behavior, &a, false);
        connect_peer(&mut behavior, &b, false);

        behavior.execute(InitiatorCommand::RequestBlocks((
            Point::Origin,
            Point::Origin,
        )));
        behavior.execute(InitiatorCommand::RequestBlocks((
            Point::Origin,
            Point::Origin,
        )));
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);

        assert_eq!(
            sends_to(&outputs, &a, is_range_request).len(),
            1,
            "one range to a"
        );
        assert_eq!(
            sends_to(&outputs, &b, is_range_request).len(),
            1,
            "one range to b"
        );
    }

    #[tokio::test]
    async fn range_request_is_not_sent_twice_before_sent() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(40);
        connect_peer(&mut behavior, &pid, false);

        behavior.execute(InitiatorCommand::RequestBlocks((
            Point::Origin,
            Point::Origin,
        )));
        behavior.execute(InitiatorCommand::RequestBlocks((
            Point::Origin,
            Point::Origin,
        )));

        behavior.execute(InitiatorCommand::Housekeeping);
        let first = sends_to(&drain_outputs(&mut behavior), &pid, is_range_request);
        assert_eq!(first.len(), 1, "first pass sends one RequestRange");

        behavior.execute(InitiatorCommand::Housekeeping);
        let second = sends_to(&drain_outputs(&mut behavior), &pid, is_range_request);
        assert_eq!(
            second.len(),
            0,
            "second pass before Sent sends no RequestRange"
        );
    }

    #[tokio::test]
    async fn held_range_request_is_sent_once_the_first_completes() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(43);
        connect_peer(&mut behavior, &pid, false);

        let held = (Point::new(2, vec![0xE2; 32]), Point::new(2, vec![0xE2; 32]));
        behavior.execute(InitiatorCommand::RequestBlocks((
            Point::Origin,
            Point::Origin,
        )));
        behavior.execute(InitiatorCommand::RequestBlocks(held.clone()));

        behavior.execute(InitiatorCommand::Housekeeping);
        let first = sends_to(&drain_outputs(&mut behavior), &pid, is_range_request);
        assert_eq!(first.len(), 1, "first pass sends one RequestRange");

        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), first[0].clone()));
        let none = AnyMessage::BlockFetch(bf::Message::NoBlocks);
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![none]));
        drain_outputs(&mut behavior);

        behavior.execute(InitiatorCommand::Housekeeping);
        let next = sends_to(&drain_outputs(&mut behavior), &pid, is_range_request);
        assert_eq!(next.len(), 1, "one RequestRange once the first completes");
        assert!(
            matches!(
                &next[0],
                AnyMessage::BlockFetch(bf::Message::RequestRange(range)) if *range == held
            ),
            "the held range goes once the first completes, got {next:?}"
        );
    }

    #[test]
    fn sent_does_not_apply_a_send_again() {
        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(44);
        let mut state = InitiatorState::new();
        let ping = AnyMessage::KeepAlive(keepalive::Message::KeepAlive(9));

        send_to_peer(&pid, &mut state, ping.clone(), &mut behavior.outbound);
        behavior.peers.insert(pid.clone(), state);
        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), ping));
        assert!(
            !behavior.peers[&pid].violation,
            "Sent does not apply the send a second time"
        );
    }

    fn assert_reply_accepted(
        behavior: &mut InitiatorBehavior,
        pid: &PeerId,
        sent: Vec<AnyMessage>,
        reply: AnyMessage,
    ) {
        for msg in sent {
            behavior.handle_io(InterfaceEvent::Sent(pid.clone(), msg));
        }
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![reply]));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(behavior);

        let state = &behavior.peers[pid];
        assert!(!state.violation, "the reply is a valid transition");
        assert_ne!(
            state.promotion,
            PromotionTag::Banned,
            "the peer is not banned"
        );
    }

    #[tokio::test]
    async fn request_range_sends_once_and_accepts_the_reply() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(45);
        connect_peer(&mut behavior, &pid, false);

        let InitiatorBehavior {
            blockfetch,
            peers,
            outbound,
            ..
        } = &mut behavior;
        let state = peers.get_mut(&pid).unwrap();
        blockfetch.request_range(&pid, state, (Point::Origin, Point::Origin), outbound);

        let sent = sends_to(&drain_outputs(&mut behavior), &pid, is_range_request);
        assert_eq!(sent.len(), 1, "one RequestRange");

        let reply = AnyMessage::BlockFetch(bf::Message::NoBlocks);
        assert_reply_accepted(&mut behavior, &pid, sent, reply);
    }

    #[tokio::test]
    async fn find_intersect_sends_once_and_accepts_the_reply() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(46);
        connect_peer(&mut behavior, &pid, false);

        let InitiatorBehavior {
            chainsync,
            peers,
            outbound,
            ..
        } = &mut behavior;
        let state = peers.get_mut(&pid).unwrap();
        chainsync.find_intersect(&pid, state, &vec![Point::Origin], outbound);

        let sent = sends_to(&drain_outputs(&mut behavior), &pid, is_find_intersect);
        assert_eq!(sent.len(), 1, "one FindIntersect");

        let tip = cs::Tip(Point::Origin, 0);
        let reply = AnyMessage::ChainSync(cs::Message::IntersectFound(Point::Origin, tip));
        assert_reply_accepted(&mut behavior, &pid, sent, reply);
    }

    #[tokio::test]
    async fn request_keepalive_sends_once_and_accepts_the_reply() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(47);
        connect_peer(&mut behavior, &pid, false);

        let InitiatorBehavior {
            keepalive,
            peers,
            outbound,
            ..
        } = &mut behavior;
        let state = peers.get_mut(&pid).unwrap();
        keepalive.request_keepalive(&pid, state, outbound);

        let sent = sends_to(&drain_outputs(&mut behavior), &pid, is_keepalive);
        assert_eq!(sent.len(), 1, "one KeepAlive");

        let AnyMessage::KeepAlive(keepalive::Message::KeepAlive(cookie)) = sent[0] else {
            unreachable!("filtered to keepalive requests");
        };
        let reply = AnyMessage::KeepAlive(keepalive::Message::ResponseKeepAlive(cookie));
        assert_reply_accepted(&mut behavior, &pid, sent, reply);
    }

    #[tokio::test]
    async fn apply_msg_before_a_raw_send_accepts_the_reply() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(48);
        connect_peer(&mut behavior, &pid, false);

        let request = AnyMessage::PeerSharing(peersharing::Message::ShareRequest(3));
        behavior.peers.get_mut(&pid).unwrap().apply_msg(&request);
        behavior
            .outbound
            .push_ready(InterfaceCommand::Send(pid.clone(), request));

        let sent = sends_to(&drain_outputs(&mut behavior), &pid, is_share_request);
        assert_eq!(sent.len(), 1, "one ShareRequest");

        let reply = AnyMessage::PeerSharing(peersharing::Message::SharePeers(vec![]));
        assert_reply_accepted(&mut behavior, &pid, sent, reply);
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn on_outbound_msg_applies_a_raw_send() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(49);
        connect_peer(&mut behavior, &pid, false);

        let request = AnyMessage::PeerSharing(peersharing::Message::ShareRequest(3));
        behavior
            .outbound
            .push_ready(InterfaceCommand::Send(pid.clone(), request.clone()));
        drain_outputs(&mut behavior);
        behavior.on_outbound_msg(&pid, &request);

        let reply = AnyMessage::PeerSharing(peersharing::Message::SharePeers(vec![]));
        assert_reply_accepted(&mut behavior, &pid, vec![], reply);
    }

    #[tokio::test]
    async fn sent_range_request_is_not_given_to_another_peer() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let a = PeerId::test(41);
        let b = PeerId::test(42);
        connect_peer(&mut behavior, &a, false);

        behavior.execute(InitiatorCommand::RequestBlocks((
            Point::Origin,
            Point::Origin,
        )));
        behavior.execute(InitiatorCommand::Housekeeping);
        let handed = sends_to(&drain_outputs(&mut behavior), &a, is_range_request);
        assert_eq!(handed.len(), 1, "a gets the RequestRange");
        behavior.handle_io(InterfaceEvent::Sent(a.clone(), handed[0].clone()));
        drain_outputs(&mut behavior);

        connect_peer(&mut behavior, &b, false);
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(&mut behavior);
        assert_eq!(
            sends_to(&outputs, &b, is_range_request).len(),
            0,
            "b gets nothing"
        );
    }

    #[tokio::test]
    async fn eb_request_is_sent_again_once_answered() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(45);
        connect_peer(&mut behavior, &pid, true);

        let first_eb = Point::new(7, vec![0xA1; 32]);
        let second_eb = Point::new(8, vec![0xA2; 32]);
        behavior.execute(InitiatorCommand::FetchEb(pid.clone(), first_eb.clone()));
        let first = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(first.len(), 1, "the fetch is sent when issued");

        behavior.execute(InitiatorCommand::FetchEb(pid.clone(), second_eb.clone()));
        drain_outputs(&mut behavior);
        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), first[0].clone()));
        let body = AnyMessage::LeiosFetch(lf::Message::Block(AnyCbor::from_raw_bytes(vec![1])));
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![body]));
        drain_outputs(&mut behavior);

        behavior.execute(InitiatorCommand::Housekeeping);
        let next = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(
            next.len(),
            1,
            "the queued fetch goes once the first is answered"
        );
        assert!(
            matches!(
                &next[0],
                AnyMessage::LeiosFetch(lf::Message::BlockRequest(eb)) if *eb == second_eb
            ),
            "the queued fetch is the second EB, got {next:?}"
        );
    }

    #[tokio::test]
    async fn eb_request_is_not_sent_twice_before_sent() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(46);
        connect_peer(&mut behavior, &pid, true);

        behavior.execute(InitiatorCommand::FetchEb(
            pid.clone(),
            Point::new(7, vec![0xB1; 32]),
        ));
        behavior.execute(InitiatorCommand::FetchEb(
            pid.clone(),
            Point::new(8, vec![0xB2; 32]),
        ));
        let issued = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(
            issued.len(),
            1,
            "two fetches issued before Sent send one request"
        );

        behavior.execute(InitiatorCommand::Housekeeping);
        let swept = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(swept.len(), 0, "housekeeping before Sent sends no request");
    }

    #[tokio::test]
    async fn unsent_eb_request_is_not_resent_after_an_error() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(47);
        connect_peer(&mut behavior, &pid, true);

        behavior.execute(InitiatorCommand::FetchEb(
            pid.clone(),
            Point::new(7, vec![0xC1; 32]),
        ));
        let issued = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(issued.len(), 1, "the fetch is sent when issued");

        fail_and_reconnect(&mut behavior, &pid);

        behavior.execute(InitiatorCommand::Housekeeping);
        let swept = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(
            swept.len(),
            0,
            "a fetch of the failed session is not resent"
        );

        behavior.execute(InitiatorCommand::FetchEb(
            pid.clone(),
            Point::new(8, vec![0xC2; 32]),
        ));
        let fresh = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(fresh.len(), 1, "a fetch on the new session is sent");
    }

    #[tokio::test]
    async fn queued_eb_request_is_purged_on_error() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(48);
        connect_peer(&mut behavior, &pid, true);

        behavior.execute(InitiatorCommand::FetchEb(
            pid.clone(),
            Point::new(7, vec![0xD1; 32]),
        ));
        let issued = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(issued.len(), 1, "the fetch is sent when issued");
        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), issued[0].clone()));

        behavior.execute(InitiatorCommand::FetchEb(
            pid.clone(),
            Point::new(8, vec![0xD2; 32]),
        ));
        let queued = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(queued.len(), 0, "a busy peer queues the second fetch");

        fail_and_reconnect(&mut behavior, &pid);

        behavior.execute(InitiatorCommand::Housekeeping);
        let swept = sends_to(&drain_outputs(&mut behavior), &pid, is_eb_request);
        assert_eq!(
            swept.len(),
            0,
            "the queued fetch was purged with the session"
        );
    }

    fn fail_and_reconnect(behavior: &mut InitiatorBehavior, pid: &PeerId) {
        behavior.handle_io(InterfaceEvent::Error(
            pid.clone(),
            InterfaceError::Other("send failed".into()),
        ));
        drain_outputs(behavior);
        behavior.handle_io(InterfaceEvent::Disconnected(pid.clone()));
        drain_outputs(behavior);

        behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        let outputs = drain_outputs(behavior);
        assert!(
            outputs.has_connect_for(pid),
            "the failed peer is connected again"
        );
        behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
        drain_outputs(behavior);
        complete_handshake_leios(behavior, pid);
        assert!(behavior.peers.get(pid).unwrap().supports_leios());
    }

    #[tokio::test]
    async fn reply_before_its_sent_is_accepted() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(50);
        connect_peer(&mut behavior, &pid, false);

        behavior.execute(InitiatorCommand::RequestBlocks((
            Point::Origin,
            Point::Origin,
        )));
        behavior.execute(InitiatorCommand::Housekeeping);
        let sent = sends_to(&drain_outputs(&mut behavior), &pid, is_range_request);
        assert_eq!(sent.len(), 1, "one RequestRange");

        let none = AnyMessage::BlockFetch(bf::Message::NoBlocks);
        behavior.handle_io(InterfaceEvent::Recv(pid.clone(), vec![none]));
        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), sent[0].clone()));
        behavior.execute(InitiatorCommand::Housekeeping);
        drain_outputs(&mut behavior);

        let state = &behavior.peers[&pid];
        assert!(!state.violation, "a reply before its Sent is valid");
        assert_ne!(
            state.promotion,
            PromotionTag::Banned,
            "the peer is not banned"
        );
    }

    #[tokio::test]
    async fn late_sent_after_a_reconnect_leaves_the_new_session_free() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(51);
        connect_peer(&mut behavior, &pid, true);

        behavior.execute(InitiatorCommand::RequestBlocks((
            Point::Origin,
            Point::Origin,
        )));
        behavior.execute(InitiatorCommand::Housekeeping);
        let old = sends_to(&drain_outputs(&mut behavior), &pid, is_range_request);
        assert_eq!(old.len(), 1, "the first session sends one RequestRange");

        fail_and_reconnect(&mut behavior, &pid);
        behavior.handle_io(InterfaceEvent::Sent(pid.clone(), old[0].clone()));
        drain_outputs(&mut behavior);

        behavior.execute(InitiatorCommand::RequestBlocks((
            Point::Origin,
            Point::Origin,
        )));
        behavior.execute(InitiatorCommand::Housekeeping);
        let fresh = sends_to(&drain_outputs(&mut behavior), &pid, is_range_request);
        assert_eq!(fresh.len(), 1, "the new session sends its RequestRange");
    }

    #[tokio::test]
    async fn second_connected_before_sent_sends_one_propose() {
        tokio::time::pause();

        let mut behavior = InitiatorBehavior::default();
        let pid = PeerId::test(52);

        for _ in 0..2 {
            behavior.execute(InitiatorCommand::IncludePeer(pid.clone()));
            behavior.execute(InitiatorCommand::Housekeeping);
            drain_outputs(&mut behavior);
        }

        let is_propose =
            |m: &AnyMessage| matches!(m, AnyMessage::Handshake(handshake::Message::Propose(_)));
        let mut proposals = 0;
        for _ in 0..2 {
            behavior.handle_io(InterfaceEvent::Connected(pid.clone()));
            proposals += sends_to(&drain_outputs(&mut behavior), &pid, is_propose).len();
        }
        assert_eq!(proposals, 1, "two Connected before Sent send one Propose");
    }
}
