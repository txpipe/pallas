use crate::{InterfaceCommand, OutboundQueue, PeerId, behavior::AnyMessage};

use super::{InitiatorBehavior, InitiatorState, PeerVisitor, send_to_peer};

/// Sub-behavior that sends periodic keepalive messages to maintain connections.
pub struct KeepaliveBehavior {
    token: u16,
}

impl Default for KeepaliveBehavior {
    fn default() -> Self {
        Self { token: u16::MAX }
    }
}

impl KeepaliveBehavior {
    /// Sends a keepalive message to the peer if the protocol state allows it.
    #[deprecated(since = "1.5.0", note = "use `request_keepalive` instead")]
    pub fn send_keepalive(
        &mut self,
        pid: &PeerId,
        peer: &InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if let Some(msg) = self.next_keepalive(peer) {
            outbound.push_ready(InterfaceCommand::Send(pid.clone(), msg));
        }
    }

    /// Sends a keepalive message to the peer if its state allows it and applies it to that state.
    pub fn request_keepalive(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if let Some(msg) = self.next_keepalive(state) {
            send_to_peer(pid, state, msg, outbound);
        }
    }

    fn next_keepalive(&self, peer: &InitiatorState) -> Option<AnyMessage> {
        if !peer.is_initialized() {
            return None;
        }

        if !matches!(peer.keepalive, crate::protocol::keepalive::State::Client(_)) {
            return None;
        }

        let msg = crate::protocol::keepalive::Message::KeepAlive(self.token);

        Some(AnyMessage::KeepAlive(msg))
    }
}

impl PeerVisitor for KeepaliveBehavior {
    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.request_keepalive(pid, state, outbound);
    }
}
