use std::collections::HashMap;
use std::task::Poll;

use futures::{Stream, StreamExt, stream::FusedStream};

use crate::{
    Behavior, BehaviorOutput, InterfaceCommand, Message as MessageTrait, OutboundQueue, PeerId,
    protocol as proto,
};

use super::{AcceptedVersion, AnyMessage, BlockRange, ConnectionState};

pub mod blockfetch;
pub mod chainsync;
pub mod connection;
pub mod handshake;
pub mod keepalive;
pub mod peersharing;
pub mod txsubmission;

pub trait ResponderPeerVisitor {
    #[allow(unused_variables)]
    fn visit_connected(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
    }

    #[allow(unused_variables)]
    fn visit_disconnected(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
    }

    #[allow(unused_variables)]
    fn visit_errored(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
    }

    #[allow(unused_variables)]
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
    }

    #[allow(unused_variables)]
    fn visit_outbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
    }

    #[allow(unused_variables)]
    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
    }
}

#[derive(Default, Debug)]
pub struct ResponderState {
    pub(crate) connection: ConnectionState,
    pub(crate) handshake: proto::handshake::State<proto::handshake::n2n::VersionData>,
    pub(crate) keepalive: proto::keepalive::State,
    pub(crate) peersharing: proto::peersharing::State,
    pub(crate) blockfetch: proto::blockfetch::State,
    pub(crate) chainsync: proto::chainsync::State<proto::chainsync::HeaderContent>,
    pub(crate) tx_submission: proto::txsubmission::State,
    pub(crate) violation: bool,
    pub(crate) error_count: u32,
    pub(crate) violations_counter: Option<opentelemetry::metrics::Counter<u64>>,
}

impl ResponderState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_violations_counter(
        mut self,
        counter: opentelemetry::metrics::Counter<u64>,
    ) -> Self {
        self.violations_counter = Some(counter);
        self
    }

    pub fn is_initialized(&self) -> bool {
        matches!(self.connection, ConnectionState::Initialized)
    }

    pub fn version(&self) -> Option<proto::handshake::n2n::VersionData> {
        match &self.handshake {
            proto::handshake::State::Done(proto::handshake::DoneState::Accepted(_, data)) => {
                Some(data.clone())
            }
            _ => None,
        }
    }

    fn record_violation(&self, protocol: &'static str) {
        if let Some(counter) = &self.violations_counter {
            counter.add(1, &[opentelemetry::KeyValue::new("protocol", protocol)]);
        }
    }

    pub fn apply_msg(&mut self, msg: &AnyMessage) {
        match msg {
            AnyMessage::Handshake(msg) => {
                let result = self.handshake.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("handshake violation");
                    self.violation = true;
                    self.record_violation("handshake");
                    return;
                };

                self.handshake = new;
            }
            AnyMessage::KeepAlive(msg) => {
                let result = self.keepalive.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("keepalive violation");
                    self.violation = true;
                    self.record_violation("keepalive");
                    return;
                };

                self.keepalive = new;
            }
            AnyMessage::PeerSharing(msg) => {
                let result = self.peersharing.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("peer sharing violation");
                    self.violation = true;
                    self.record_violation("peersharing");
                    return;
                };

                self.peersharing = new;
            }
            AnyMessage::BlockFetch(msg) => {
                let result = self.blockfetch.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("block fetch violation");
                    self.violation = true;
                    self.record_violation("blockfetch");
                    return;
                };

                self.blockfetch = new;
            }
            AnyMessage::ChainSync(msg) => {
                let result = self.chainsync.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("chain sync violation");
                    self.violation = true;
                    self.record_violation("chainsync");
                    return;
                };

                self.chainsync = new;
            }
            AnyMessage::TxSubmission(msg) => {
                let result = self.tx_submission.apply(msg);

                let Ok(new) = result else {
                    tracing::warn!("tx submission violation");
                    self.violation = true;
                    self.record_violation("txsubmission");
                    return;
                };

                self.tx_submission = new;
            }
        }
    }

    pub fn reset(&mut self) {
        self.connection = ConnectionState::default();
        self.handshake = proto::handshake::State::default();
        self.keepalive = proto::keepalive::State::default();
        self.peersharing = proto::peersharing::State::default();
        self.blockfetch = proto::blockfetch::State::default();
        self.chainsync = proto::chainsync::State::default();
        self.tx_submission = proto::txsubmission::State::default();
        self.violation = false;
    }
}

#[derive(Debug)]
pub enum ResponderCommand {
    Housekeeping,
    ProvideIntersection(PeerId, proto::Point, proto::chainsync::Tip),
    ProvideHeader(
        PeerId,
        proto::chainsync::HeaderContent,
        proto::chainsync::Tip,
    ),
    ProvideRollback(PeerId, proto::Point, proto::chainsync::Tip),
    ProvideBlocks(PeerId, Vec<proto::blockfetch::Body>),
    ProvidePeers(PeerId, Vec<proto::peersharing::PeerAddress>),
    BanPeer(PeerId),
    DisconnectPeer(PeerId),
}

#[derive(Debug)]
pub enum ResponderEvent {
    PeerInitialized(PeerId, AcceptedVersion),
    IntersectionRequested(PeerId, Vec<proto::Point>),
    NextHeaderRequested(PeerId),
    BlockRangeRequested(PeerId, BlockRange),
    PeersRequested(PeerId, u8),
    TxReceived(PeerId, proto::txsubmission::EraTxBody),
}

pub struct ResponderBehavior {
    pub connection: connection::ConnectionResponder,
    pub handshake: handshake::HandshakeResponder,
    pub keepalive: keepalive::KeepaliveResponder,
    pub chainsync: chainsync::ChainSyncResponder,
    pub blockfetch: blockfetch::BlockFetchResponder,
    pub peersharing: peersharing::PeerSharingResponder,
    pub txsubmission: txsubmission::TxSubmissionResponder,
    pub peers: HashMap<PeerId, ResponderState>,
    pub outbound: OutboundQueue<Self>,

    // metrics
    pub violations_counter: opentelemetry::metrics::Counter<u64>,
}

impl Default for ResponderBehavior {
    fn default() -> Self {
        let meter = opentelemetry::global::meter("pallas-network2");

        let violations_counter = meter
            .u64_counter("responder_protocol_violations")
            .with_description("Protocol violations by type")
            .build();

        Self {
            connection: Default::default(),
            handshake: Default::default(),
            keepalive: Default::default(),
            chainsync: Default::default(),
            blockfetch: Default::default(),
            peersharing: Default::default(),
            txsubmission: Default::default(),
            peers: Default::default(),
            outbound: Default::default(),
            violations_counter,
        }
    }
}


macro_rules! all_visitors {
    ($self:ident, $pid:ident, $state:expr, $method:ident) => {
        $self.connection.$method($pid, $state, &mut $self.outbound);
        $self.handshake.$method($pid, $state, &mut $self.outbound);
        $self.keepalive.$method($pid, $state, &mut $self.outbound);
        $self.chainsync.$method($pid, $state, &mut $self.outbound);
        $self.blockfetch.$method($pid, $state, &mut $self.outbound);
        $self.peersharing.$method($pid, $state, &mut $self.outbound);
        $self.txsubmission.$method($pid, $state, &mut $self.outbound);
    };
}

impl ResponderBehavior {
    #[tracing::instrument(skip_all, fields(pid = %pid, channel = %msg.channel()))]
    pub fn on_inbound_msg(&mut self, pid: &PeerId, msg: &AnyMessage) {
        tracing::debug!(channel = msg.channel(), "new inbound message");

        self.peers.entry(pid.clone()).and_modify(|state| {
            state.apply_msg(msg);

            // Dispatch only to the visitor that owns the inbound message's
            // protocol.  The previous `all_visitors!` call triggered every
            // visitor on every message, which caused duplicate responses
            // (e.g. multiple ResponseKeepAlive) when several mini-protocol
            // messages arrived in quick succession.
            self.connection.visit_inbound_msg(pid, state, &mut self.outbound);
            match msg {
                AnyMessage::Handshake(_) => {
                    self.handshake.visit_inbound_msg(pid, state, &mut self.outbound);
                }
                AnyMessage::KeepAlive(_) => {
                    self.keepalive.visit_inbound_msg(pid, state, &mut self.outbound);
                }
                AnyMessage::ChainSync(_) => {
                    self.chainsync.visit_inbound_msg(pid, state, &mut self.outbound);
                }
                AnyMessage::BlockFetch(_) => {
                    self.blockfetch.visit_inbound_msg(pid, state, &mut self.outbound);
                }
                AnyMessage::PeerSharing(_) => {
                    self.peersharing.visit_inbound_msg(pid, state, &mut self.outbound);
                }
                AnyMessage::TxSubmission(_) => {
                    self.txsubmission.visit_inbound_msg(pid, state, &mut self.outbound);
                }
            }
        });
    }

    #[tracing::instrument(skip_all, fields(pid = %pid, channel = %msg.channel()))]
    pub fn on_outbound_msg(&mut self, pid: &PeerId, msg: &AnyMessage) {
        tracing::debug!(channel = msg.channel(), "new outbound message");

        self.peers.entry(pid.clone()).and_modify(|state| {
            state.apply_msg(msg);

            all_visitors!(self, pid, state, visit_outbound_msg);
        });
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_connected(&mut self, pid: &PeerId) {
        tracing::info!("responder: peer connected");

        let mut state =
            ResponderState::new().with_violations_counter(self.violations_counter.clone());
        state.connection = ConnectionState::Connected;

        all_visitors!(self, pid, &mut state, visit_connected);

        self.peers.insert(pid.clone(), state);
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_disconnected(&mut self, pid: &PeerId) {
        tracing::info!("responder: peer disconnected");

        self.peers.entry(pid.clone()).and_modify(|state| {
            state.connection = ConnectionState::Disconnected;
            state.reset();

            all_visitors!(self, pid, state, visit_disconnected);
        });
    }

    #[tracing::instrument(skip_all, fields(pid = %pid))]
    fn on_errored(&mut self, pid: &PeerId) {
        tracing::error!("responder: peer error");

        self.peers.entry(pid.clone()).and_modify(|state| {
            state.connection = ConnectionState::Errored;
            state.error_count += 1;

            all_visitors!(self, pid, state, visit_errored);
        });
    }

    #[tracing::instrument(skip_all)]
    fn housekeeping(&mut self) {
        for (pid, state) in self.peers.iter_mut() {
            all_visitors!(self, pid, state, visit_housekeeping);
        }
    }

    fn provide_intersection(
        &mut self,
        pid: &PeerId,
        point: proto::Point,
        tip: proto::chainsync::Tip,
    ) {
        let msg = proto::chainsync::Message::IntersectFound(point, tip);
        self.outbound.push_ready(BehaviorOutput::InterfaceCommand(
            InterfaceCommand::Send(pid.clone(), AnyMessage::ChainSync(msg)),
        ));
    }

    fn provide_header(
        &mut self,
        pid: &PeerId,
        header: proto::chainsync::HeaderContent,
        tip: proto::chainsync::Tip,
    ) {
        let msg = proto::chainsync::Message::RollForward(header, tip);
        self.outbound.push_ready(BehaviorOutput::InterfaceCommand(
            InterfaceCommand::Send(pid.clone(), AnyMessage::ChainSync(msg)),
        ));
    }

    fn provide_rollback(
        &mut self,
        pid: &PeerId,
        point: proto::Point,
        tip: proto::chainsync::Tip,
    ) {
        let msg = proto::chainsync::Message::RollBackward(point, tip);
        self.outbound.push_ready(BehaviorOutput::InterfaceCommand(
            InterfaceCommand::Send(pid.clone(), AnyMessage::ChainSync(msg)),
        ));
    }

    fn provide_blocks(&mut self, pid: &PeerId, blocks: Vec<proto::blockfetch::Body>) {
        // Send StartBatch
        self.outbound.push_ready(BehaviorOutput::InterfaceCommand(
            InterfaceCommand::Send(
                pid.clone(),
                AnyMessage::BlockFetch(proto::blockfetch::Message::StartBatch),
            ),
        ));

        // Send each block
        for block in blocks {
            self.outbound.push_ready(BehaviorOutput::InterfaceCommand(
                InterfaceCommand::Send(
                    pid.clone(),
                    AnyMessage::BlockFetch(proto::blockfetch::Message::Block(block)),
                ),
            ));
        }

        // Send BatchDone
        self.outbound.push_ready(BehaviorOutput::InterfaceCommand(
            InterfaceCommand::Send(
                pid.clone(),
                AnyMessage::BlockFetch(proto::blockfetch::Message::BatchDone),
            ),
        ));
    }

    fn provide_peers(&mut self, pid: &PeerId, peers: Vec<proto::peersharing::PeerAddress>) {
        let msg = proto::peersharing::Message::SharePeers(peers);
        self.outbound.push_ready(BehaviorOutput::InterfaceCommand(
            InterfaceCommand::Send(pid.clone(), AnyMessage::PeerSharing(msg)),
        ));
    }

    fn ban_peer(&mut self, pid: &PeerId) {
        self.connection.banned_peers.insert(pid.clone());
        self.outbound.push_ready(BehaviorOutput::InterfaceCommand(
            InterfaceCommand::Disconnect(pid.clone()),
        ));
    }

    fn disconnect_peer(&mut self, pid: &PeerId) {
        self.outbound.push_ready(BehaviorOutput::InterfaceCommand(
            InterfaceCommand::Disconnect(pid.clone()),
        ));
    }
}

impl Stream for ResponderBehavior {
    type Item = BehaviorOutput<Self>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let poll = self.outbound.futures.poll_next_unpin(cx);

        match poll {
            Poll::Ready(Some(x)) => Poll::Ready(Some(x)),
            _ => Poll::Pending,
        }
    }
}

impl FusedStream for ResponderBehavior {
    fn is_terminated(&self) -> bool {
        false
    }
}

impl Behavior for ResponderBehavior {
    type Event = ResponderEvent;
    type Command = ResponderCommand;
    type PeerState = ResponderState;
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
            ResponderCommand::Housekeeping => {
                tracing::debug!("housekeeping command");
                self.housekeeping();
            }
            ResponderCommand::ProvideIntersection(pid, point, tip) => {
                tracing::debug!("provide intersection command");
                self.provide_intersection(&pid, point, tip);
            }
            ResponderCommand::ProvideHeader(pid, header, tip) => {
                tracing::debug!("provide header command");
                self.provide_header(&pid, header, tip);
            }
            ResponderCommand::ProvideRollback(pid, point, tip) => {
                tracing::debug!("provide rollback command");
                self.provide_rollback(&pid, point, tip);
            }
            ResponderCommand::ProvideBlocks(pid, blocks) => {
                tracing::debug!("provide blocks command");
                self.provide_blocks(&pid, blocks);
            }
            ResponderCommand::ProvidePeers(pid, peers) => {
                tracing::debug!("provide peers command");
                self.provide_peers(&pid, peers);
            }
            ResponderCommand::BanPeer(pid) => {
                tracing::debug!("ban peer command");
                self.ban_peer(&pid);
            }
            ResponderCommand::DisconnectPeer(pid) => {
                tracing::debug!("disconnect peer command");
                self.disconnect_peer(&pid);
            }
        }
    }
}
