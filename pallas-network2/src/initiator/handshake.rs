use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    initiator::{AnyMessage, InitiatorBehavior, InitiatorEvent, InitiatorState, PeerVisitor},
    protocol::MAINNET_MAGIC,
};

pub struct Config {
    supported_version: crate::protocol::handshake::n2n::VersionTable,
}

pub struct HandshakeBehavior {
    config: Config,
}

impl Default for HandshakeBehavior {
    fn default() -> Self {
        Self::new(Config {
            supported_version: crate::protocol::handshake::n2n::VersionTable {
                values: vec![(
                    13,
                    crate::protocol::handshake::n2n::VersionData {
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
            state.handshake,
            crate::protocol::handshake::State::Propose
        ));

        let msg =
            crate::protocol::handshake::Message::Propose(self.config.supported_version.clone());

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
        let crate::protocol::handshake::State::Done(
            crate::protocol::handshake::DoneState::Accepted(num, data),
        ) = &state.handshake
        else {
            return;
        };

        state.connection = crate::initiator::ConnectionState::Initialized;

        let out = BehaviorOutput::ExternalEvent(InitiatorEvent::PeerInitialized(
            pid.clone(),
            (*num, data.clone()),
        ));

        outbound.push_ready(out);
    }
}

fn needs_handshake(peer: &InitiatorState) -> bool {
    matches!(
        peer.connection,
        crate::initiator::ConnectionState::Connected
    )
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
        if needs_handshake(state) {
            self.check_confirmation(pid, state, outbound);
        }
    }
}
