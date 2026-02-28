use std::collections::{HashMap, HashSet};

use crate::{
    InterfaceCommand, OutboundQueue, PeerId,
    behavior::ConnectionState,
};

use super::{ResponderBehavior, ResponderPeerVisitor, ResponderState};

pub struct ConnectionResponderConfig {
    pub max_error_count: u32,
    pub max_connections_per_ip: usize,
}

impl Default for ConnectionResponderConfig {
    fn default() -> Self {
        Self {
            max_error_count: 1,
            max_connections_per_ip: 10,
        }
    }
}

pub struct ConnectionResponder {
    config: ConnectionResponderConfig,
    pub(crate) banned_peers: HashSet<PeerId>,
    connections_per_ip: HashMap<String, usize>,
    active_peers: usize,

    // metrics
    active_peers_gauge: opentelemetry::metrics::Gauge<u64>,
    connections_accepted_counter: opentelemetry::metrics::Counter<u64>,
    connections_rejected_counter: opentelemetry::metrics::Counter<u64>,
    peers_banned_counter: opentelemetry::metrics::Counter<u64>,
}

impl Default for ConnectionResponder {
    fn default() -> Self {
        Self::new(ConnectionResponderConfig::default())
    }
}

impl ConnectionResponder {
    pub fn new(config: ConnectionResponderConfig) -> Self {
        let meter = opentelemetry::global::meter("pallas-network2");

        let active_peers_gauge = meter
            .u64_gauge("responder_active_peers")
            .with_description("Current active responder peer count")
            .build();

        let connections_accepted_counter = meter
            .u64_counter("responder_connections_accepted")
            .with_description("Total accepted responder connections")
            .build();

        let connections_rejected_counter = meter
            .u64_counter("responder_connections_rejected")
            .with_description("Responder connections rejected (too many per IP)")
            .build();

        let peers_banned_counter = meter
            .u64_counter("responder_peers_banned")
            .with_description("Total responder peers banned")
            .build();

        Self {
            config,
            banned_peers: HashSet::new(),
            connections_per_ip: HashMap::new(),
            active_peers: 0,
            active_peers_gauge,
            connections_accepted_counter,
            connections_rejected_counter,
            peers_banned_counter,
        }
    }

    fn needs_disconnect(&self, pid: &PeerId, state: &ResponderState) -> bool {
        if self.banned_peers.contains(pid) {
            return true;
        }

        matches!(state.connection, ConnectionState::Errored)
    }

    fn needs_ban(&self, pid: &PeerId, state: &ResponderState) -> bool {
        if self.banned_peers.contains(pid) {
            return false;
        }

        state.violation || state.error_count > self.config.max_error_count
    }
}

impl ResponderPeerVisitor for ConnectionResponder {
    fn visit_connected(
        &mut self,
        pid: &PeerId,
        _state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        let count = self.connections_per_ip.entry(pid.host.clone()).or_insert(0);
        *count += 1;

        if *count > self.config.max_connections_per_ip {
            tracing::warn!(
                ip = %pid.host,
                count = *count,
                max = self.config.max_connections_per_ip,
                "too many connections from IP, disconnecting"
            );
            self.connections_rejected_counter.add(1, &[]);
            outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
        } else {
            self.active_peers += 1;
            self.active_peers_gauge.record(self.active_peers as u64, &[]);
            self.connections_accepted_counter.add(1, &[]);
        }
    }

    fn visit_disconnected(
        &mut self,
        pid: &PeerId,
        _state: &mut ResponderState,
        _outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if let Some(count) = self.connections_per_ip.get_mut(&pid.host) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.connections_per_ip.remove(&pid.host);
            }
        }

        self.active_peers = self.active_peers.saturating_sub(1);
        self.active_peers_gauge.record(self.active_peers as u64, &[]);
    }

    fn visit_errored(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if self.needs_disconnect(pid, state) {
            tracing::info!("disconnecting errored responder peer");
            outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
        }
    }

    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if self.needs_ban(pid, state) {
            tracing::warn!("banning misbehaving responder peer");
            self.banned_peers.insert(pid.clone());
            self.peers_banned_counter.add(1, &[]);
            outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
            return;
        }

        if self.needs_disconnect(pid, state) {
            tracing::info!("disconnecting responder peer");
            outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
        }
    }
}
