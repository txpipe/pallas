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
}

impl Default for ConnectionResponder {
    fn default() -> Self {
        Self::new(ConnectionResponderConfig::default())
    }
}

impl ConnectionResponder {
    pub fn new(config: ConnectionResponderConfig) -> Self {
        Self {
            config,
            banned_peers: HashSet::new(),
            connections_per_ip: HashMap::new(),
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
            outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
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
            outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
            return;
        }

        if self.needs_disconnect(pid, state) {
            tracing::info!("disconnecting responder peer");
            outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
        }
    }
}
