use crate::{
    InterfaceCommand, OutboundQueue, PeerId,
    behavior::{ConnectionState, InitiatorBehavior, InitiatorState, PeerVisitor, PromotionTag},
};

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

#[derive(Debug)]
pub struct ConnectionConfig {}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {}
    }
}

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
