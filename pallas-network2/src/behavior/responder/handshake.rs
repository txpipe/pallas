use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{AnyMessage, ConnectionState},
    protocol::{MAINNET_MAGIC, handshake as handshake_proto},
};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

pub struct HandshakeResponderConfig {
    pub supported_version: handshake_proto::n2n::VersionTable,
}

pub struct HandshakeResponder {
    config: HandshakeResponderConfig,
}

impl Default for HandshakeResponder {
    fn default() -> Self {
        Self::new(HandshakeResponderConfig {
            supported_version: handshake_proto::n2n::VersionTable {
                values: vec![(
                    13,
                    handshake_proto::n2n::VersionData {
                        network_magic: MAINNET_MAGIC,
                        initiator_only_diffusion_mode: false,
                        peer_sharing: Some(1),
                        query: Some(false),
                    },
                )]
                .into_iter()
                .collect(),
            },
        })
    }
}

impl HandshakeResponder {
    pub fn new(config: HandshakeResponderConfig) -> Self {
        Self { config }
    }

    fn try_accept_handshake(
        &self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        let handshake_proto::State::Confirm(proposed) = &state.handshake else {
            return;
        };

        // Find highest common version
        let negotiated = proposed
            .values
            .iter()
            .filter(|(num, _)| self.config.supported_version.values.contains_key(num))
            .max_by_key(|(num, _)| *num)
            .map(|(num, data)| (*num, data.clone()));

        match negotiated {
            Some((version, data)) => {
                tracing::info!(version, "accepting handshake");

                let msg = handshake_proto::Message::Accept(version, data.clone());
                outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    pid.clone(),
                    AnyMessage::Handshake(msg),
                )));

                state.connection = ConnectionState::Initialized;

                outbound.push_ready(BehaviorOutput::ExternalEvent(
                    ResponderEvent::PeerInitialized(pid.clone(), (version, data)),
                ));
            }
            None => {
                tracing::warn!("refusing handshake: no common version");

                let our_versions: Vec<u64> =
                    self.config.supported_version.values.keys().copied().collect();

                let msg = handshake_proto::Message::Refuse(
                    handshake_proto::RefuseReason::VersionMismatch(our_versions),
                );
                outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    pid.clone(),
                    AnyMessage::Handshake(msg),
                )));
            }
        }
    }
}

impl ResponderPeerVisitor for HandshakeResponder {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        self.try_accept_handshake(pid, state, outbound);
    }
}
