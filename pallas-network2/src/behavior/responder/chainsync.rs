use crate::{
    BehaviorOutput, OutboundQueue, PeerId,
    protocol::chainsync as chainsync_proto,
};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

pub struct ChainSyncResponder {
    // metrics
    chainsync_requests_counter: opentelemetry::metrics::Counter<u64>,
}

impl Default for ChainSyncResponder {
    fn default() -> Self {
        let meter = opentelemetry::global::meter("pallas-network2");

        let chainsync_requests_counter = meter
            .u64_counter("responder_chainsync_requests")
            .with_description("Total chainsync header requests served")
            .build();

        Self {
            chainsync_requests_counter,
        }
    }
}

impl ChainSyncResponder {
    fn check_requests(
        &self,
        pid: &PeerId,
        state: &ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if !state.is_initialized() {
            return;
        }

        match &state.chainsync {
            chainsync_proto::State::Intersect(points) => {
                tracing::debug!("intersection requested");
                outbound.push_ready(BehaviorOutput::ExternalEvent(
                    ResponderEvent::IntersectionRequested(pid.clone(), points.clone()),
                ));
            }
            chainsync_proto::State::CanAwait | chainsync_proto::State::MustReply => {
                tracing::debug!("next header requested");
                self.chainsync_requests_counter.add(1, &[]);
                outbound.push_ready(BehaviorOutput::ExternalEvent(
                    ResponderEvent::NextHeaderRequested(pid.clone()),
                ));
            }
            _ => {}
        }
    }
}

impl ResponderPeerVisitor for ChainSyncResponder {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        self.check_requests(pid, state, outbound);
    }
}
