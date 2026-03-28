use crate::{InterfaceCommand, OutboundQueue, PeerId, behavior::ConnectionState};

use super::{InitiatorBehavior, InitiatorState, PeerVisitor, PromotionTag};

fn needs_connection(peer: &InitiatorState) -> bool {
    match peer.connection {
        ConnectionState::Connected => false,
        ConnectionState::Connecting => false,
        ConnectionState::Initialized => false,
        _ => match peer.promotion {
            PromotionTag::Warm => true,
            PromotionTag::Hot => true,
            PromotionTag::Banned => false,
            PromotionTag::Cold => false,
        },
    }
}

fn needs_disconnect(peer: &InitiatorState) -> bool {
    match peer.connection {
        ConnectionState::Errored => true,
        ConnectionState::New => false,
        ConnectionState::Connecting => false,
        ConnectionState::Disconnected => false,
        ConnectionState::Connected | ConnectionState::Initialized => match peer.promotion {
            PromotionTag::Cold => true,
            PromotionTag::Banned => true,
            PromotionTag::Warm => false,
            PromotionTag::Hot => false,
        },
    }
}

#[derive(Debug, Default)]
pub struct ConnectionConfig {}

pub struct ConnectionBehavior {
    _config: ConnectionConfig,

    // metrics
    connection_counter: opentelemetry::metrics::Counter<u64>,
}

impl Default for ConnectionBehavior {
    fn default() -> Self {
        Self::new(ConnectionConfig::default())
    }
}

impl ConnectionBehavior {
    pub fn new(config: ConnectionConfig) -> Self {
        let meter = opentelemetry::global::meter("pallas-network2");

        let connection_counter = meter
            .u64_counter("connections")
            .with_description("Total connections")
            .build();

        Self {
            _config: config,
            connection_counter,
        }
    }

    fn increment_counter(&self) {
        self.connection_counter.add(1, &[]);
    }
}

impl PeerVisitor for ConnectionBehavior {
    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if needs_connection(state) {
            state.connection = ConnectionState::Connecting;
            outbound.push_ready(InterfaceCommand::Connect(pid.clone()));
            self.increment_counter();
        }

        if needs_disconnect(state) {
            tracing::info!("disconnecting peer");
            outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
        }
    }

    fn visit_errored(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if needs_disconnect(state) {
            tracing::info!("disconnecting errored peer");
            outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with(connection: ConnectionState, promotion: PromotionTag) -> InitiatorState {
        let mut s = InitiatorState::new();
        s.connection = connection;
        s.promotion = promotion;
        s
    }

    // --- needs_connection tests ---

    #[test]
    fn warm_new_needs_connection() {
        let s = state_with(ConnectionState::New, PromotionTag::Warm);
        assert!(needs_connection(&s));
    }

    #[test]
    fn hot_disconnected_needs_connection() {
        let s = state_with(ConnectionState::Disconnected, PromotionTag::Hot);
        assert!(needs_connection(&s));
    }

    #[test]
    fn cold_does_not_connect() {
        let s = state_with(ConnectionState::New, PromotionTag::Cold);
        assert!(!needs_connection(&s));
    }

    #[test]
    fn banned_does_not_connect() {
        let s = state_with(ConnectionState::Disconnected, PromotionTag::Banned);
        assert!(!needs_connection(&s));
    }

    #[test]
    fn already_connected_does_not_reconnect() {
        let s = state_with(ConnectionState::Connected, PromotionTag::Warm);
        assert!(!needs_connection(&s));
    }

    // --- needs_disconnect tests ---

    #[test]
    fn errored_needs_disconnect() {
        let s = state_with(ConnectionState::Errored, PromotionTag::Warm);
        assert!(needs_disconnect(&s));
    }

    #[test]
    fn cold_initialized_needs_disconnect() {
        let s = state_with(ConnectionState::Initialized, PromotionTag::Cold);
        assert!(needs_disconnect(&s));
    }

    #[test]
    fn hot_initialized_stays_connected() {
        let s = state_with(ConnectionState::Initialized, PromotionTag::Hot);
        assert!(!needs_disconnect(&s));
    }
}
