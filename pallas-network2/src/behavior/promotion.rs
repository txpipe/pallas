use std::collections::HashSet;

use crate::{
    OutboundQueue, PeerId,
    behavior::{InitiatorBehavior, InitiatorState, PeerVisitor, Promotion},
};

impl From<crate::protocol::peersharing::PeerAddress> for PeerId {
    fn from(addr: crate::protocol::peersharing::PeerAddress) -> Self {
        match addr {
            crate::protocol::peersharing::PeerAddress::V4(addr, port) => PeerId {
                host: addr.to_string(),
                port: port as u16,
            },
            crate::protocol::peersharing::PeerAddress::V6(addr, port) => PeerId {
                host: addr.to_string(),
                port: port as u16,
            },
        }
    }
}

#[derive(Debug)]
pub struct PromotionConfig {
    max_peers: usize,
    max_warm_peers: usize,
    max_hot_peers: usize,
    max_error_count: u32,
}

impl Default for PromotionConfig {
    fn default() -> Self {
        Self {
            max_peers: 100,
            max_warm_peers: 50,
            max_hot_peers: 10,
            max_error_count: 1,
        }
    }
}

pub struct PromotionBehavior {
    config: PromotionConfig,
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
        Self::new(PromotionConfig::default())
    }
}

impl PromotionBehavior {
    pub fn new(config: PromotionConfig) -> Self {
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

    pub fn peer_deficit(&self) -> usize {
        self.config.max_peers - self.total_peers()
    }

    fn total_peers(&self) -> usize {
        self.cold_peers.len() + self.warm_peers.len() + self.hot_peers.len()
    }

    fn required_cold_peers(&self) -> usize {
        self.config.max_peers - self.total_peers()
    }

    fn required_warm_peers(&self) -> usize {
        self.config.max_warm_peers - self.warm_peers.len()
    }

    fn required_hot_peers(&self) -> usize {
        self.config.max_hot_peers - self.hot_peers.len()
    }

    fn promote_warm_peer(&mut self, pid: &PeerId, peer: &mut InitiatorState) {
        if let Some(x) = self.warm_peers.take(pid) {
            tracing::info!(%pid, "promoting warm peer");

            self.hot_peers.insert(x);
            self.update_metrics();

            peer.promotion = Promotion::Hot;
        }
    }

    fn promote_cold_peer(&mut self, pid: &PeerId, peer: &mut InitiatorState) {
        if let Some(x) = self.cold_peers.take(pid) {
            tracing::info!(%pid, "promoting cold peer");

            self.warm_peers.insert(x);
            self.update_metrics();

            peer.promotion = Promotion::Warm;
        }
    }

    fn ban_peer(&mut self, pid: &PeerId, peer: &mut InitiatorState) {
        tracing::warn!("banning peer");

        self.hot_peers.remove(pid);
        self.warm_peers.remove(pid);
        self.cold_peers.remove(pid);
        self.banned_peers.insert(pid.clone());
        self.update_metrics();

        peer.promotion = Promotion::Banned;
    }

    fn categorize_peer(&mut self, pid: &PeerId, peer: &mut InitiatorState) {
        if peer.violation && !self.banned_peers.contains(pid) {
            self.ban_peer(pid, peer);
            return;
        }

        if peer.error_count > self.config.max_error_count && !self.banned_peers.contains(pid) {
            self.ban_peer(pid, peer);
            return;
        }

        if self.required_warm_peers() > 0 && self.cold_peers.contains(pid) {
            self.promote_cold_peer(pid, peer);
            return;
        }

        if self.required_hot_peers() > 0 && self.warm_peers.contains(pid) && peer.is_initialized() {
            self.promote_warm_peer(pid, peer);
            return;
        }
    }

    pub fn on_peer_discovered(&mut self, pid: &PeerId, state: &mut InitiatorState) {
        if self.banned_peers.contains(pid) {
            tracing::warn!("skipping discovered peer, already banned");
            return;
        }

        if self.required_cold_peers() > 0 {
            tracing::debug!("flagging peer as cold");
            self.cold_peers.insert(pid.clone());

            state.promotion = Promotion::Cold;
        } else {
            tracing::warn!("discovered peer, but max peers reached");
        }
    }
}

impl PeerVisitor for PromotionBehavior {
    fn visit_discovered(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        _: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.on_peer_discovered(pid, state);
    }

    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        _: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.categorize_peer(pid, state);
    }

    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        _: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.categorize_peer(pid, state);
    }
}
