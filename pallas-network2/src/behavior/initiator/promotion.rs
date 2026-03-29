use std::collections::HashSet;

use crate::{OutboundQueue, PeerId};

use super::{InitiatorBehavior, InitiatorState, PeerVisitor, PromotionTag};

impl From<crate::protocol::peersharing::PeerAddress> for PeerId {
    fn from(addr: crate::protocol::peersharing::PeerAddress) -> Self {
        match addr {
            crate::protocol::peersharing::PeerAddress::V4(addr, port) => PeerId {
                host: addr.to_string(),
                port,
            },
            crate::protocol::peersharing::PeerAddress::V6(addr, port) => PeerId {
                host: addr.to_string(),
                port,
            },
        }
    }
}

#[derive(Debug)]
pub struct PromotionConfig {
    pub max_peers: usize,
    pub max_warm_peers: usize,
    pub max_hot_peers: usize,
    pub max_error_count: u32,
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

            peer.promotion = PromotionTag::Hot;
        }
    }

    fn promote_cold_peer(&mut self, pid: &PeerId, peer: &mut InitiatorState) {
        if let Some(x) = self.cold_peers.take(pid) {
            tracing::info!(%pid, "promoting cold peer");

            self.warm_peers.insert(x);
            self.update_metrics();

            peer.promotion = PromotionTag::Warm;
        }
    }

    pub fn ban_peer(&mut self, pid: &PeerId, peer: &mut InitiatorState) {
        tracing::warn!("banning peer");

        self.hot_peers.remove(pid);
        self.warm_peers.remove(pid);
        self.cold_peers.remove(pid);
        self.banned_peers.insert(pid.clone());
        self.update_metrics();

        peer.promotion = PromotionTag::Banned;
    }

    pub fn demote_peer(&mut self, pid: &PeerId, peer: &mut InitiatorState) {
        tracing::warn!("demoting peer");

        if self.banned_peers.contains(pid) {
            tracing::warn!("cannot demote banned peer");
            return;
        }

        self.hot_peers.remove(pid);
        self.warm_peers.remove(pid);
        self.cold_peers.insert(pid.clone());

        self.update_metrics();

        peer.promotion = PromotionTag::Cold;
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

            state.promotion = PromotionTag::Cold;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_warm_peers_respected() {
        let mut promo = PromotionBehavior::new(PromotionConfig {
            max_peers: 100,
            max_warm_peers: 2,
            max_hot_peers: 10,
            max_error_count: 5,
        });

        // Discover 5 peers (all land in cold)
        for i in 1..=5 {
            let pid = PeerId::test(i);
            let mut state = InitiatorState::new();
            promo.on_peer_discovered(&pid, &mut state);
        }
        assert_eq!(promo.cold_peers.len(), 5);

        // categorize_peer should promote at most 2 to warm
        let peers: Vec<PeerId> = promo.cold_peers.iter().cloned().collect();
        for pid in &peers {
            let mut state = InitiatorState::new();
            state.promotion = PromotionTag::Cold;
            promo.categorize_peer(pid, &mut state);
        }

        assert!(
            promo.warm_peers.len() <= 2,
            "warm_peers ({}) should not exceed max_warm_peers (2)",
            promo.warm_peers.len()
        );
    }

    #[test]
    fn max_hot_peers_respected() {
        let mut promo = PromotionBehavior::new(PromotionConfig {
            max_peers: 100,
            max_warm_peers: 10,
            max_hot_peers: 1,
            max_error_count: 5,
        });

        // Put 3 peers directly into warm with initialized state
        for i in 1..=3 {
            let pid = PeerId::test(i);
            promo.warm_peers.insert(pid);
        }

        let peers: Vec<PeerId> = promo.warm_peers.iter().cloned().collect();
        for pid in &peers {
            let mut state = InitiatorState::new();
            state.connection = crate::behavior::ConnectionState::Initialized;
            state.promotion = PromotionTag::Warm;
            promo.categorize_peer(pid, &mut state);
        }

        assert!(
            promo.hot_peers.len() <= 1,
            "hot_peers ({}) should not exceed max_hot_peers (1)",
            promo.hot_peers.len()
        );
    }

    #[test]
    fn max_peers_caps_total() {
        let mut promo = PromotionBehavior::new(PromotionConfig {
            max_peers: 3,
            max_warm_peers: 10,
            max_hot_peers: 10,
            max_error_count: 5,
        });

        for i in 1..=5 {
            let pid = PeerId::test(i);
            let mut state = InitiatorState::new();
            promo.on_peer_discovered(&pid, &mut state);
        }

        let total =
            promo.cold_peers.len() + promo.warm_peers.len() + promo.hot_peers.len();

        assert!(
            total <= 3,
            "total tracked peers ({}) should not exceed max_peers (3)",
            total
        );
    }

    #[test]
    fn peer_deficit_reflects_current_count() {
        let mut promo = PromotionBehavior::new(PromotionConfig {
            max_peers: 10,
            max_warm_peers: 10,
            max_hot_peers: 10,
            max_error_count: 5,
        });

        for i in 1..=7 {
            let pid = PeerId::test(i);
            let mut state = InitiatorState::new();
            promo.on_peer_discovered(&pid, &mut state);
        }

        assert_eq!(promo.peer_deficit(), 3);
    }

    #[test]
    fn ban_peer_removes_from_all_sets() {
        let mut promo = PromotionBehavior::new(PromotionConfig::default());
        let pid = PeerId::test(1);

        promo.warm_peers.insert(pid.clone());

        let mut state = InitiatorState::new();
        promo.ban_peer(&pid, &mut state);

        assert!(promo.banned_peers.contains(&pid));
        assert!(!promo.warm_peers.contains(&pid));
        assert_eq!(state.promotion, PromotionTag::Banned);
    }

    #[test]
    fn demote_banned_peer_is_noop() {
        let mut promo = PromotionBehavior::new(PromotionConfig::default());
        let pid = PeerId::test(1);

        promo.banned_peers.insert(pid.clone());

        let mut state = InitiatorState::new();
        state.promotion = PromotionTag::Banned;
        promo.demote_peer(&pid, &mut state);

        assert!(
            promo.banned_peers.contains(&pid),
            "banned peer should remain banned after demote"
        );
        assert!(
            !promo.cold_peers.contains(&pid),
            "banned peer should not appear in cold_peers"
        );
    }

    #[test]
    fn rediscovered_banned_peer_stays_banned() {
        let mut promo = PromotionBehavior::new(PromotionConfig::default());
        let pid = PeerId::test(1);

        promo.banned_peers.insert(pid.clone());

        let mut state = InitiatorState::new();
        promo.on_peer_discovered(&pid, &mut state);

        assert!(promo.banned_peers.contains(&pid));
        assert!(!promo.cold_peers.contains(&pid));
    }

    #[test]
    fn violation_triggers_ban() {
        let mut promo = PromotionBehavior::new(PromotionConfig::default());
        let pid = PeerId::test(1);

        promo.warm_peers.insert(pid.clone());

        let mut state = InitiatorState::new();
        state.violation = true;
        promo.categorize_peer(&pid, &mut state);

        assert!(promo.banned_peers.contains(&pid));
    }

    #[test]
    fn error_count_exceeding_max_triggers_ban() {
        let mut promo = PromotionBehavior::new(PromotionConfig {
            max_error_count: 1,
            ..PromotionConfig::default()
        });
        let pid = PeerId::test(1);

        promo.warm_peers.insert(pid.clone());

        // error_count = 1, not > 1 → no ban
        let mut state = InitiatorState::new();
        state.error_count = 1;
        promo.categorize_peer(&pid, &mut state);
        assert!(!promo.banned_peers.contains(&pid));

        // error_count = 2 > 1 → ban
        state.error_count = 2;
        promo.categorize_peer(&pid, &mut state);
        assert!(promo.banned_peers.contains(&pid));
    }
}
