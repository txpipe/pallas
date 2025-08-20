use pallas_network::miniprotocols::{Agent as _, keepalive as keepalive_proto};

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{AnyMessage, InitiatorState},
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
    pub fn on_peer_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if matches!(state.keepalive.state(), keepalive_proto::State::Client(_)) {
            let msg = keepalive_proto::Message::KeepAlive(self.token);

            let out = BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                pid.clone(),
                AnyMessage::KeepAlive(msg),
            ));

            outbound.push_ready(out);
        }
    }
}
