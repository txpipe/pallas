use crate::{InterfaceCommand, OutboundQueue, PeerId, behavior::AnyMessage};

use super::{InitiatorBehavior, InitiatorState, PeerVisitor};

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

        if matches!(peer.keepalive, crate::protocol::keepalive::State::Client(_)) {
            let msg = crate::protocol::keepalive::Message::KeepAlive(self.token);

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
