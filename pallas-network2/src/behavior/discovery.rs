use std::collections::HashSet;

use pallas_network::miniprotocols::{Agent as _, peersharing as peersharing_proto};

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{AnyMessage, InitiatorState},
};

pub struct Config {
    request_amount: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self { request_amount: 5 }
    }
}

#[derive(Default)]
pub struct DiscoveryBehavior {
    config: Config,
    discovered: HashSet<PeerId>,
}

impl DiscoveryBehavior {
    fn request_peers(&self, pid: &PeerId, outbound: &mut OutboundQueue<super::InitiatorBehavior>) {
        let msg = peersharing_proto::Message::ShareRequest(self.config.request_amount);

        let out = BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::PeerSharing(msg),
        ));

        outbound.push_ready(out);
    }

    pub fn visit_updated_peer(
        &mut self,
        pid: &PeerId,
        peer: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if peer.is_initialized() && peer.supports_peer_sharing() {
            match peer.peersharing.state() {
                peersharing_proto::State::Idle(peersharing_proto::IdleState::Empty) => {
                    self.request_peers(pid, outbound);
                }
                peersharing_proto::State::Idle(peersharing_proto::IdleState::Response(peers)) => {
                    for peer in peers {
                        self.discovered.insert(peer.clone().into());
                    }

                    peer.peersharing =
                        peersharing_proto::Client::new(peersharing_proto::State::Done);
                }
                _ => (),
            }
        }
    }

    pub fn take_peers(&mut self) -> Vec<PeerId> {
        self.discovered.drain().collect()
    }
}
