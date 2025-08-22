use pallas_network::miniprotocols::{Agent as _, MAINNET_MAGIC, handshake as handshake_proto};

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{
        AcceptedVersion, AnyMessage, InitiatorBehavior, InitiatorEvent, InitiatorState, PeerVisitor,
    },
};

pub struct Config {
    supported_version: handshake_proto::n2n::VersionTable,
}

pub struct HandshakeBehavior {
    config: Config,
}

impl Default for HandshakeBehavior {
    fn default() -> Self {
        Self::new(Config {
            supported_version: pallas_network::miniprotocols::handshake::n2n::VersionTable {
                values: vec![(
                    13,
                    pallas_network::miniprotocols::handshake::n2n::VersionData {
                        network_magic: MAINNET_MAGIC,
                        initiator_only_diffusion_mode: false,
                        peer_sharing: Some(1),
                        //peer_sharing: Some(0),
                        query: Some(false),
                    },
                )]
                .into_iter()
                .collect(),
            },
        })
    }
}

impl HandshakeBehavior {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    fn propose_handshake(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        assert!(matches!(
            state.handshake.state(),
            handshake_proto::State::Propose
        ));

        let msg = handshake_proto::Message::Propose(self.config.supported_version.clone());

        let out = BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::Handshake(msg),
        ));

        outbound.push_ready(out);
    }

    fn check_confirmation(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        let handshake_proto::State::Done(handshake_proto::DoneState::Accepted(num, data)) =
            state.handshake.state()
        else {
            return;
        };

        state.connection = crate::behavior::ConnectionState::Initialized;

        let out = BehaviorOutput::ExternalEvent(InitiatorEvent::PeerInitialized(
            pid.clone(),
            (*num, data.clone()),
        ));

        outbound.push_ready(out);
    }
}

fn needs_handshake(peer: &InitiatorState) -> bool {
    matches!(peer.connection, crate::behavior::ConnectionState::Connected)
}

impl PeerVisitor for HandshakeBehavior {
    fn visit_connected(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.propose_handshake(pid, state, outbound);
    }

    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // TODO: more efficient if we could subscribe just for messages of the
        // appropriate protocol
        if needs_handshake(state) {
            self.check_confirmation(pid, state, outbound);
        }
    }
}
