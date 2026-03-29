use std::collections::HashSet;

use crate::{BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId, behavior::AnyMessage};

use super::{InitiatorBehavior, InitiatorState, PeerVisitor};

/// Configuration for the peer discovery sub-behavior.
pub struct DiscoveryConfig {
    /// Maximum number of discovered peers to accumulate before stopping requests.
    pub high_water_mark: u8,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            high_water_mark: 100,
        }
    }
}

fn peer_supports_peer_sharing(peer: &InitiatorState) -> bool {
    peer.is_initialized() && peer.supports_peer_sharing()
}

fn peer_is_available(peer: &InitiatorState) -> bool {
    peer_supports_peer_sharing(peer)
        && matches!(
            &peer.peersharing,
            crate::protocol::peersharing::State::Idle(
                crate::protocol::peersharing::IdleState::Empty
            )
        )
}

/// Sub-behavior that discovers new peers via the peer-sharing mini-protocol.
#[derive(Default)]
pub struct DiscoveryBehavior {
    config: DiscoveryConfig,
    discovered: HashSet<PeerId>,
}

impl DiscoveryBehavior {
    fn request_peers(&self, pid: &PeerId, outbound: &mut OutboundQueue<super::InitiatorBehavior>) {
        let amount = self.config.high_water_mark as usize - self.discovered.len();

        tracing::debug!(amount, "requesting peers");

        let msg = crate::protocol::peersharing::Message::ShareRequest(amount as u8);

        let out = BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::PeerSharing(msg),
        ));

        outbound.push_ready(out);
    }

    /// Extracts discovered peer addresses from the peer-sharing response, if
    /// available.
    pub fn try_take_peers(&mut self, peer: &mut InitiatorState) {
        if let crate::protocol::peersharing::State::Idle(
            crate::protocol::peersharing::IdleState::Response(peers),
        ) = &peer.peersharing
        {
            tracing::info!(peers = peers.len(), "got peer discovery response");

            for peer in peers {
                self.discovered.insert(peer.clone().into());
            }

            // TODO: think of how we reset this after a while to ask again
            // for peers to the same responder.
            peer.peersharing = crate::protocol::peersharing::State::Done;
        }
    }

    /// Takes up to `count` discovered peers out of the internal pool.
    pub fn drain_new_peers(&mut self, count: usize) -> HashSet<PeerId> {
        let selected: HashSet<_> = self.discovered.iter().take(count).cloned().collect();

        self.discovered = self.discovered.difference(&selected).cloned().collect();

        tracing::debug!(
            count = selected.len(),
            remaining = self.discovered.len(),
            "peers drained"
        );

        selected
    }

    fn needs_more_peers(&self) -> bool {
        self.discovered.len() < self.config.high_water_mark as usize
    }
}

impl PeerVisitor for DiscoveryBehavior {
    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if !self.needs_more_peers() {
            return;
        }

        if !peer_is_available(state) {
            return;
        }

        self.request_peers(pid, outbound);
    }

    fn visit_inbound_msg(
        &mut self,
        _: &PeerId,
        state: &mut InitiatorState,
        _: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if peer_supports_peer_sharing(state) {
            self.try_take_peers(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior::ConnectionState;
    use crate::protocol::{MAINNET_MAGIC, handshake, peersharing};
    use std::net::Ipv4Addr;

    fn make_initialized_peer_sharing_state() -> InitiatorState {
        let mut s = InitiatorState::new();
        s.connection = ConnectionState::Initialized;
        // Set handshake to Done(Accepted) with peer_sharing > 0
        let vd = handshake::n2n::VersionData::new(MAINNET_MAGIC, false, Some(1), Some(false));
        s.handshake = handshake::State::Done(handshake::DoneState::Accepted(13, vd));
        s
    }

    #[test]
    fn try_take_peers_extracts_from_response() {
        let mut disc = DiscoveryBehavior::default();
        let mut state = make_initialized_peer_sharing_state();

        // Set peersharing state to a response with 2 peers
        state.peersharing = peersharing::State::Idle(peersharing::IdleState::Response(vec![
            peersharing::PeerAddress::V4(Ipv4Addr::new(1, 2, 3, 4), 3000),
            peersharing::PeerAddress::V4(Ipv4Addr::new(5, 6, 7, 8), 3001),
        ]));

        disc.try_take_peers(&mut state);

        assert_eq!(disc.discovered.len(), 2);
        assert!(matches!(state.peersharing, peersharing::State::Done));
    }

    #[test]
    fn drain_returns_up_to_count() {
        let mut disc = DiscoveryBehavior {
            config: DiscoveryConfig {
                high_water_mark: 100,
            },
            discovered: HashSet::new(),
        };

        // Insert 5 peers
        for i in 1..=5 {
            disc.discovered.insert(PeerId {
                host: format!("10.0.0.{}", i),
                port: 3000,
            });
        }

        let drained = disc.drain_new_peers(2);
        assert_eq!(drained.len(), 2);
        assert_eq!(disc.discovered.len(), 3);
    }

    #[test]
    fn high_water_mark_stops_requests() {
        let mut disc = DiscoveryBehavior {
            config: DiscoveryConfig { high_water_mark: 3 },
            discovered: HashSet::new(),
        };

        // Fill to high water mark
        for i in 1..=3 {
            disc.discovered.insert(PeerId {
                host: format!("10.0.0.{}", i),
                port: 3000,
            });
        }

        assert!(!disc.needs_more_peers());
    }

    #[test]
    fn needs_more_peers_when_below_mark() {
        let disc = DiscoveryBehavior {
            config: DiscoveryConfig {
                high_water_mark: 10,
            },
            discovered: HashSet::new(),
        };

        assert!(disc.needs_more_peers());
    }
}
