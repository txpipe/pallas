use std::collections::HashSet;

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{AnyMessage, InitiatorBehavior, InitiatorState, PeerVisitor},
};

pub struct Config {
    high_water_mark: u8,
}

impl Default for Config {
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

#[derive(Default)]
pub struct DiscoveryBehavior {
    config: Config,
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

    pub fn try_take_peers(&mut self, peer: &mut InitiatorState) {
        match &peer.peersharing {
            crate::protocol::peersharing::State::Idle(
                crate::protocol::peersharing::IdleState::Response(peers),
            ) => {
                tracing::info!(peers = peers.len(), "got peer discovery response");

                for peer in peers {
                    self.discovered.insert(peer.clone().into());
                }

                // TODO: think of how we reset this after a while to ask again
                // for peers to the same responder.
                peer.peersharing = crate::protocol::peersharing::State::Done;
            }
            _ => (),
        }
    }

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
