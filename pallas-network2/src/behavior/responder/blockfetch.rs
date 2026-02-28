use crate::{BehaviorOutput, OutboundQueue, PeerId, protocol::blockfetch as blockfetch_proto};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

pub struct BlockFetchResponder {
    // metrics
    blockfetch_requests_counter: opentelemetry::metrics::Counter<u64>,
}

impl Default for BlockFetchResponder {
    fn default() -> Self {
        let meter = opentelemetry::global::meter("pallas-network2");

        let blockfetch_requests_counter = meter
            .u64_counter("responder_blockfetch_requests")
            .with_description("Total block range requests served")
            .build();

        Self {
            blockfetch_requests_counter,
        }
    }
}

impl BlockFetchResponder {
    fn check_requests(
        &self,
        pid: &PeerId,
        state: &ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if !state.is_initialized() {
            return;
        }

        if let blockfetch_proto::State::Busy(range) = &state.blockfetch {
            tracing::debug!("block range requested");
            self.blockfetch_requests_counter.add(1, &[]);
            outbound.push_ready(BehaviorOutput::ExternalEvent(
                ResponderEvent::BlockRangeRequested(pid.clone(), range.clone()),
            ));
        }
    }
}

impl ResponderPeerVisitor for BlockFetchResponder {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        self.check_requests(pid, state, outbound);
    }
}
