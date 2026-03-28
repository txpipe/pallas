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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutboundQueue;
    use futures::StreamExt;
    use std::collections::HashMap;

    fn make_peer() -> PeerId {
        PeerId {
            host: "10.0.0.1".to_string(),
            port: 3001,
        }
    }

    fn drain_outputs(
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) -> Vec<BehaviorOutput<ResponderBehavior>> {
        let mut outputs = Vec::new();
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        loop {
            match outbound.futures.poll_next_unpin(&mut cx) {
                std::task::Poll::Ready(Some(output)) => outputs.push(output),
                _ => break,
            }
        }

        outputs
    }

    fn make_version_data(magic: u64) -> handshake_proto::n2n::VersionData {
        handshake_proto::n2n::VersionData::new(magic, false, Some(1), Some(false))
    }

    fn make_responder_with_versions(versions: Vec<(u64, u64)>) -> HandshakeResponder {
        let values: HashMap<u64, handshake_proto::n2n::VersionData> = versions
            .into_iter()
            .map(|(num, magic)| (num, make_version_data(magic)))
            .collect();

        HandshakeResponder::new(HandshakeResponderConfig {
            supported_version: handshake_proto::n2n::VersionTable { values },
        })
    }

    fn make_proposed_table(versions: Vec<(u64, u64)>) -> handshake_proto::VersionTable<handshake_proto::n2n::VersionData> {
        let values: HashMap<u64, handshake_proto::n2n::VersionData> = versions
            .into_iter()
            .map(|(num, magic)| (num, make_version_data(magic)))
            .collect();

        handshake_proto::VersionTable { values }
    }

    #[test]
    fn accepts_highest_common_version() {
        // We support v13, v14. Peer proposes v12, v13, v14.
        let mut hs = make_responder_with_versions(vec![(13, MAINNET_MAGIC), (14, MAINNET_MAGIC)]);
        let pid = make_peer();
        let mut state = ResponderState::new();
        let mut outbound = OutboundQueue::new();

        state.handshake = handshake_proto::State::Confirm(
            make_proposed_table(vec![(12, MAINNET_MAGIC), (13, MAINNET_MAGIC), (14, MAINNET_MAGIC)]),
        );

        hs.visit_inbound_msg(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let accepted_version = outputs.iter().find_map(|o| match o {
            BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                _,
                AnyMessage::Handshake(handshake_proto::Message::Accept(v, _)),
            )) => Some(*v),
            _ => None,
        });

        assert_eq!(accepted_version, Some(14), "should accept highest common version");
        assert_eq!(state.connection, ConnectionState::Initialized);
    }

    #[test]
    fn refuses_no_common_version() {
        // We support v13. Peer proposes v7, v8.
        let mut hs = make_responder_with_versions(vec![(13, MAINNET_MAGIC)]);
        let pid = make_peer();
        let mut state = ResponderState::new();
        let mut outbound = OutboundQueue::new();

        state.handshake = handshake_proto::State::Confirm(
            make_proposed_table(vec![(7, MAINNET_MAGIC), (8, MAINNET_MAGIC)]),
        );

        hs.visit_inbound_msg(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let has_refuse = outputs.iter().any(|o| {
            matches!(
                o,
                BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    _,
                    AnyMessage::Handshake(handshake_proto::Message::Refuse(
                        handshake_proto::RefuseReason::VersionMismatch(_)
                    ))
                ))
            )
        });
        assert!(has_refuse, "should refuse with VersionMismatch");
    }

    #[test]
    fn refuses_magic_mismatch() {
        // We support v13 with mainnet magic. Peer proposes v13 with different magic.
        let mut hs = make_responder_with_versions(vec![(13, MAINNET_MAGIC)]);
        let pid = make_peer();
        let mut state = ResponderState::new();
        let mut outbound = OutboundQueue::new();

        state.handshake = handshake_proto::State::Confirm(
            make_proposed_table(vec![(13, 999999)]), // wrong magic
        );

        hs.visit_inbound_msg(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let has_refuse = outputs.iter().any(|o| {
            matches!(
                o,
                BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    _,
                    AnyMessage::Handshake(handshake_proto::Message::Refuse(
                        handshake_proto::RefuseReason::Refused(..)
                    ))
                ))
            )
        });
        assert!(has_refuse, "should refuse with magic mismatch");
    }

    #[test]
    fn accepted_handshake_emits_initialized_event() {
        let mut hs = make_responder_with_versions(vec![(13, MAINNET_MAGIC)]);
        let pid = make_peer();
        let mut state = ResponderState::new();
        let mut outbound = OutboundQueue::new();

        state.handshake = handshake_proto::State::Confirm(
            make_proposed_table(vec![(13, MAINNET_MAGIC)]),
        );

        hs.visit_inbound_msg(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let has_event = outputs.iter().any(|o| {
            matches!(o, BehaviorOutput::ExternalEvent(ResponderEvent::PeerInitialized(..)))
        });
        assert!(has_event, "should emit PeerInitialized event");
    }
}
