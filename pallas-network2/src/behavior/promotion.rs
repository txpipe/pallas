use std::collections::HashSet;

use pallas_network::miniprotocols::peersharing;

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{InitiatorEvent, InitiatorState},
};

impl From<peersharing::PeerAddress> for PeerId {
    fn from(addr: peersharing::PeerAddress) -> Self {
        match addr {
            peersharing::PeerAddress::V4(addr, port) => PeerId {
                host: addr.to_string(),
                port: port as u16,
            },
            peersharing::PeerAddress::V6(addr, port) => PeerId {
                host: addr.to_string(),
                port: port as u16,
            },
        }
    }
}

#[derive(Debug)]
pub struct DiscoveryConfig {
    max_peers: usize,
    max_warm_peers: usize,
    max_hot_peers: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            max_peers: 50,
            max_warm_peers: 5,
            max_hot_peers: 3,
        }
    }
}

pub struct PromotionBehavior {
    config: DiscoveryConfig,
    pub cold_peers: HashSet<PeerId>,
    pub warm_peers: HashSet<PeerId>,
    pub hot_peers: HashSet<PeerId>,
    pub banned_peers: HashSet<PeerId>,

    // metrics
    cold_peers_gauge: opentelemetry::metrics::Gauge<u64>,
    warm_peers_gauge: opentelemetry::metrics::Gauge<u64>,
    hot_peers_gauge: opentelemetry::metrics::Gauge<u64>,
    banned_peers_gauge: opentelemetry::metrics::Gauge<u64>,
}

impl Default for PromotionBehavior {
    fn default() -> Self {
        Self::new(DiscoveryConfig::default())
    }
}

impl PromotionBehavior {
    pub fn new(config: DiscoveryConfig) -> Self {
        let meter = opentelemetry::global::meter("pallas-network2");

        let cold_peers_gauge = meter
            .u64_gauge("cold_peers")
            .with_description("Total cold peers")
            .build();

        let warm_peers_gauge = meter
            .u64_gauge("warm_peers")
            .with_description("Total warm peers")
            .build();

        let hot_peers_gauge = meter
            .u64_gauge("hot_peers")
            .with_description("Total hot peers")
            .build();

        let banned_peers_gauge = meter
            .u64_gauge("banned_peers")
            .with_description("Total banned peers")
            .build();

        Self {
            config,
            cold_peers: Default::default(),
            warm_peers: Default::default(),
            hot_peers: Default::default(),
            banned_peers: Default::default(),
            cold_peers_gauge,
            warm_peers_gauge,
            hot_peers_gauge,
            banned_peers_gauge,
        }
    }

    fn update_metrics(&self) {
        self.cold_peers_gauge
            .record(self.cold_peers.len() as u64, &[]);

        self.warm_peers_gauge
            .record(self.warm_peers.len() as u64, &[]);

        self.hot_peers_gauge
            .record(self.hot_peers.len() as u64, &[]);

        self.banned_peers_gauge
            .record(self.banned_peers.len() as u64, &[]);
    }

    fn total_peers(&self) -> usize {
        self.cold_peers.len() + self.warm_peers.len() + self.hot_peers.len()
    }

    fn required_warm_peers(&self) -> usize {
        self.config.max_warm_peers - self.warm_peers.len()
    }

    fn promote_warm_peer(
        &mut self,
        pid: &PeerId,
        state: &InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if let Some(x) = self.warm_peers.take(pid) {
            tracing::info!(%pid, "promoting warm peer");
            self.hot_peers.insert(x);
            self.update_metrics();

            let version = state.version();

            if let Some(version) = version {
                outbound.push_ready(BehaviorOutput::ExternalEvent(
                    InitiatorEvent::PeerInitialized(pid.clone(), version),
                ));
            }
        }
    }

    fn promote_cold_peer(&mut self, pid: &PeerId) {
        if let Some(x) = self.cold_peers.take(pid) {
            tracing::info!(%pid, "promoting cold peer");
            self.warm_peers.insert(x);
            self.update_metrics();
        }
    }

    fn connect_warm_peer(
        &mut self,
        pid: &PeerId,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        tracing::info!(%pid, "connecting warm peer");
        outbound.push_ready(InterfaceCommand::Connect(pid.clone()));
    }

    fn disconnect_violation_peer(
        &mut self,
        pid: &PeerId,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        tracing::warn!(%pid, "disconnecting peer due to violation");
        self.hot_peers.remove(pid);
        self.warm_peers.remove(pid);
        self.cold_peers.remove(pid);
        self.banned_peers.insert(pid.clone());
        self.update_metrics();

        outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
    }

    pub fn on_peer_housekeeping(
        &mut self,
        pid: &PeerId,
        peer: &InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if peer.violation {
            self.disconnect_violation_peer(pid, outbound);
        }

        if self.cold_peers.contains(pid) && self.required_warm_peers() > 0 {
            self.promote_cold_peer(pid);
        }

        if self.warm_peers.contains(pid) && peer.needs_connection() {
            self.connect_warm_peer(pid, outbound);
        }
    }

    pub fn on_peer_discovered(&mut self, pid: &PeerId) {
        if self.total_peers() < self.config.max_peers {
            tracing::info!(%pid, "discovered new peer");
            self.cold_peers.insert(pid.clone());
        } else {
            tracing::warn!(%pid, "discovered peer, but max peers reached");
        }
    }

    pub fn visit_updated_peer(
        &mut self,
        pid: &PeerId,
        peer: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if self.hot_peers.len() < self.config.max_hot_peers
            && self.warm_peers.contains(pid)
            && peer.is_initialized()
        {
            self.promote_warm_peer(pid, peer, outbound);
        }
    }

    pub fn select_random_hot_peer(&self) -> Option<&PeerId> {
        self.hot_peers.iter().next()
    }
}
