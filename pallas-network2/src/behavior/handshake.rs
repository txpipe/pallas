use pallas_network::miniprotocols::{Agent as _, handshake as handshake_proto};

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{AnyMessage, InitiatorState},
};

pub type Config = handshake_proto::n2n::VersionTable;

pub struct HandshakeBehavior {
    config: Config,
}

impl Default for HandshakeBehavior {
    fn default() -> Self {
        Self::new(Config::v11_and_above(0))
    }
}

impl HandshakeBehavior {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn visit_updated_peer(
        &self,
        pid: &PeerId,
        state: &InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if matches!(state.handshake.state(), handshake_proto::State::Propose) {
            let msg = handshake_proto::Message::Propose(self.config.clone());

            let out = BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                pid.clone(),
                AnyMessage::Handshake(msg),
            ));

            outbound.push_ready(out);
        }
    }
}
