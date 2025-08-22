use pallas_network::miniprotocols::{Agent as _, keepalive as keepalive_proto};

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{AnyMessage, InitiatorBehavior, InitiatorState, PeerVisitor},
};

pub struct KeepaliveBehavior {
    token: u16,
}

impl Default for KeepaliveBehavior {
    fn default() -> Self {
        Self { token: u16::MAX }
    }
}

impl KeepaliveBehavior {
    pub fn send_keepalive(
        &mut self,
        pid: &PeerId,
        peer: &InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if !peer.is_initialized() {
            return;
        }

        if matches!(peer.keepalive.state(), keepalive_proto::State::Client(_)) {
            let msg = keepalive_proto::Message::KeepAlive(self.token);

            let out = InterfaceCommand::Send(pid.clone(), AnyMessage::KeepAlive(msg));

            outbound.push_ready(out);
        }
    }
}

impl PeerVisitor for KeepaliveBehavior {
    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.send_keepalive(pid, state, outbound);
    }
}
