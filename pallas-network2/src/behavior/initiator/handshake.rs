use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId, behavior::AnyMessage,
    protocol::MAINNET_MAGIC,
};

use super::{InitiatorBehavior, InitiatorEvent, InitiatorState, PeerVisitor};

/// Configuration for the handshake sub-behavior.
pub struct Config {
    /// The N2N version table to propose during handshakes.
    pub supported_version: crate::protocol::handshake::n2n::VersionTable,
}

/// Sub-behavior that performs the handshake mini-protocol on new connections.
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
    /// Creates a new handshake behavior with the given configuration.
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
        if needs_handshake(state) {
            self.check_confirmation(pid, state, outbound);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutboundQueue;
    use crate::behavior::ConnectionState;
    use crate::protocol::handshake;

    fn drain_outputs(
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) -> Vec<BehaviorOutput<InitiatorBehavior>> {
        outbound.drain_ready()
    }

    #[test]
    fn propose_sent_on_connect() {
        let mut hs = HandshakeBehavior::default();
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        hs.visit_connected(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        assert_eq!(outputs.len(), 1);

        let is_propose = outputs.iter().any(|o| {
            matches!(
                o,
                BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    _,
                    AnyMessage::Handshake(handshake::Message::Propose(_))
                ))
            )
        });
        assert!(is_propose, "should send a Propose message on connect");
    }

    #[test]
    fn accepted_handshake_sets_initialized() {
        let mut hs = HandshakeBehavior::default();
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        // Put state into Done(Accepted) as if the handshake completed
        let version_data = crate::protocol::handshake::n2n::VersionData::new(
            MAINNET_MAGIC,
            false,
            Some(1),
            Some(false),
        );
        state.handshake = handshake::State::Done(handshake::DoneState::Accepted(13, version_data));
        state.connection = ConnectionState::Connected;

        hs.visit_inbound_msg(&pid, &mut state, &mut outbound);

        assert_eq!(
            state.connection,
            ConnectionState::Initialized,
            "connection should be set to Initialized after accepted handshake"
        );

        let outputs = drain_outputs(&mut outbound);
        let has_init_event = outputs.iter().any(|o| {
            matches!(
                o,
                BehaviorOutput::ExternalEvent(InitiatorEvent::PeerInitialized(..))
            )
        });
        assert!(has_init_event, "should emit PeerInitialized event");
    }

    #[test]
    fn non_connected_state_skips_confirmation() {
        let mut hs = HandshakeBehavior::default();
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        // State is Initialized (not Connected), so needs_handshake returns false
        state.connection = ConnectionState::Initialized;

        hs.visit_inbound_msg(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        assert!(
            outputs.is_empty(),
            "should not produce output when not in Connected state"
        );
    }
}
