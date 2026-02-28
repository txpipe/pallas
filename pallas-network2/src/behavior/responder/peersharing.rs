use crate::{
    BehaviorOutput, OutboundQueue, PeerId,
    protocol::peersharing as peersharing_proto,
};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

pub struct PeerSharingResponder {
    // metrics
    peersharing_requests_counter: opentelemetry::metrics::Counter<u64>,
}

impl Default for PeerSharingResponder {
    fn default() -> Self {
        let meter = opentelemetry::global::meter("pallas-network2");

        let peersharing_requests_counter = meter
            .u64_counter("responder_peersharing_requests")
            .with_description("Total peer sharing requests")
            .build();

        Self {
            peersharing_requests_counter,
        }
    }
}

impl PeerSharingResponder {
    fn check_requests(
        &self,
        pid: &PeerId,
        state: &ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if !state.is_initialized() {
            return;
        }

        if let peersharing_proto::State::Busy(amount) = &state.peersharing {
            tracing::debug!(amount, "peers requested");
            self.peersharing_requests_counter.add(1, &[]);
            outbound.push_ready(BehaviorOutput::ExternalEvent(
                ResponderEvent::PeersRequested(pid.clone(), *amount),
            ));
        }
    }
}

impl ResponderPeerVisitor for PeerSharingResponder {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        self.check_requests(pid, state, outbound);
    }
}
