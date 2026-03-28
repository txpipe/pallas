use std::collections::{HashMap, HashSet};

use crate::{InterfaceCommand, OutboundQueue, PeerId, behavior::ConnectionState};

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
    accepted_peers: HashSet<PeerId>,
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
            accepted_peers: HashSet::new(),
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
        if self.banned_peers.contains(pid) {
            tracing::debug!(peer = %pid, "rejecting connection from banned peer");
            self.connections_rejected_counter.add(1, &[]);
            outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
            return;
        }

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
        } else if self.accepted_peers.insert(pid.clone()) {
            self.active_peers += 1;
            self.active_peers_gauge
                .record(self.active_peers as u64, &[]);
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

        if self.accepted_peers.remove(pid) {
            self.active_peers = self.active_peers.saturating_sub(1);
            self.active_peers_gauge
                .record(self.active_peers as u64, &[]);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutboundQueue;

    fn make_peer_same_ip(id: u16) -> PeerId {
        PeerId {
            host: "10.0.0.1".to_string(),
            port: 3000 + id,
        }
    }

    fn make_peer(id: u8) -> PeerId {
        PeerId {
            host: format!("10.0.0.{}", id),
            port: 3000 + id as u16,
        }
    }

    fn drain_outputs(
        outbound: &mut OutboundQueue<super::super::ResponderBehavior>,
    ) -> Vec<crate::BehaviorOutput<super::super::ResponderBehavior>> {
        use futures::StreamExt;
        let mut outputs = Vec::new();
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        loop {
            match outbound.futures.poll_next_unpin(&mut cx) {
                std::task::Poll::Ready(Some(output)) => outputs.push(output),
                _ => break,
            }
        }

        outputs
    }

    fn has_disconnect_for(
        outputs: &[crate::BehaviorOutput<super::super::ResponderBehavior>],
        pid: &PeerId,
    ) -> bool {
        outputs.iter().any(|o| {
            matches!(
                o,
                crate::BehaviorOutput::InterfaceCommand(InterfaceCommand::Disconnect(p)) if p == pid
            )
        })
    }

    #[test]
    fn per_ip_limit_enforced() {
        let mut conn = ConnectionResponder::new(ConnectionResponderConfig {
            max_error_count: 5,
            max_connections_per_ip: 3,
        });
        let mut outbound = OutboundQueue::new();

        for i in 1..=5 {
            let pid = make_peer_same_ip(i);
            let mut state = ResponderState::new();
            conn.visit_connected(&pid, &mut state, &mut outbound);
        }

        let outputs = drain_outputs(&mut outbound);

        // Peers 4 and 5 should be disconnected
        assert!(has_disconnect_for(&outputs, &make_peer_same_ip(4)));
        assert!(has_disconnect_for(&outputs, &make_peer_same_ip(5)));
        // Peers 1-3 should not
        assert!(!has_disconnect_for(&outputs, &make_peer_same_ip(1)));
        assert!(!has_disconnect_for(&outputs, &make_peer_same_ip(2)));
        assert!(!has_disconnect_for(&outputs, &make_peer_same_ip(3)));
    }

    #[test]
    fn banned_peer_rejected_on_connect() {
        let mut conn = ConnectionResponder::new(ConnectionResponderConfig::default());
        let mut outbound = OutboundQueue::new();
        let pid = make_peer(1);

        conn.banned_peers.insert(pid.clone());

        let mut state = ResponderState::new();
        conn.visit_connected(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        assert!(has_disconnect_for(&outputs, &pid));
    }

    #[test]
    fn disconnect_frees_ip_slot() {
        let mut conn = ConnectionResponder::new(ConnectionResponderConfig {
            max_error_count: 5,
            max_connections_per_ip: 3,
        });
        let mut outbound = OutboundQueue::new();

        // Fill 3 slots
        for i in 1..=3 {
            let pid = make_peer_same_ip(i);
            let mut state = ResponderState::new();
            conn.visit_connected(&pid, &mut state, &mut outbound);
        }
        drain_outputs(&mut outbound);

        // Disconnect peer 1
        let mut state = ResponderState::new();
        conn.visit_disconnected(&make_peer_same_ip(1), &mut state, &mut outbound);
        drain_outputs(&mut outbound);

        // Peer 4 should be accepted (slot freed)
        let pid4 = make_peer_same_ip(4);
        let mut state = ResponderState::new();
        conn.visit_connected(&pid4, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        assert!(!has_disconnect_for(&outputs, &pid4));
    }

    #[test]
    fn violation_leads_to_ban_on_housekeeping() {
        let mut conn = ConnectionResponder::new(ConnectionResponderConfig::default());
        let mut outbound = OutboundQueue::new();
        let pid = make_peer(2);

        let mut state = ResponderState::new();
        state.violation = true;

        conn.visit_housekeeping(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        assert!(conn.banned_peers.contains(&pid));
        assert!(has_disconnect_for(&outputs, &pid));
    }
}
