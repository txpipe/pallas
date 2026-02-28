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

    // metrics
    handshakes_completed_counter: opentelemetry::metrics::Counter<u64>,
    handshakes_refused_counter: opentelemetry::metrics::Counter<u64>,
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
        let meter = opentelemetry::global::meter("pallas-network2");

        let handshakes_completed_counter = meter
            .u64_counter("responder_handshakes_completed")
            .with_description("Successful responder handshakes")
            .build();

        let handshakes_refused_counter = meter
            .u64_counter("responder_handshakes_refused")
            .with_description("Refused responder handshakes (version mismatch)")
            .build();

        Self {
            config,
            handshakes_completed_counter,
            handshakes_refused_counter,
        }
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
            .map(|(num, peer_data)| {
                let our_data = &self.config.supported_version.values[num];
                (*num, peer_data.clone(), our_data.clone())
            });

        match negotiated {
            Some((version, peer_data, our_data)) => {
                if peer_data.network_magic != our_data.network_magic {
                    tracing::warn!(
                        peer_magic = peer_data.network_magic,
                        our_magic = our_data.network_magic,
                        "refusing handshake: network magic mismatch"
                    );
                    self.handshakes_refused_counter.add(1, &[]);
                    let msg =
                        handshake_proto::Message::Refuse(handshake_proto::RefuseReason::Refused(
                            version,
                            "network magic mismatch".to_string(),
                        ));
                    outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                        pid.clone(),
                        AnyMessage::Handshake(msg),
                    )));
                    return;
                }

                tracing::info!(version, "accepting handshake");

                let msg = handshake_proto::Message::Accept(version, our_data.clone());
                outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    pid.clone(),
                    AnyMessage::Handshake(msg),
                )));

                state.connection = ConnectionState::Initialized;
                self.handshakes_completed_counter.add(1, &[]);

                outbound.push_ready(BehaviorOutput::ExternalEvent(
                    ResponderEvent::PeerInitialized(pid.clone(), (version, our_data)),
                ));
            }
            None => {
                tracing::warn!("refusing handshake: no common version");
                self.handshakes_refused_counter.add(1, &[]);

                let our_versions: Vec<u64> = self
                    .config
                    .supported_version
                    .values
                    .keys()
                    .copied()
                    .collect();

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
