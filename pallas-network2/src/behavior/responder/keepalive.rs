use crate::{OutboundQueue, PeerId, behavior::AnyMessage, protocol::keepalive as keepalive_proto};

use super::{ResponderBehavior, ResponderPeerVisitor, ResponderState, send_to_peer};

/// Responder sub-behavior that automatically replies to keepalive requests.
pub struct KeepaliveResponder {
    // metrics
    keepalive_responses_counter: opentelemetry::metrics::Counter<u64>,
}

impl Default for KeepaliveResponder {
    fn default() -> Self {
        let meter = opentelemetry::global::meter("pallas-network2");

        let keepalive_responses_counter = meter
            .u64_counter("responder_keepalive_responses")
            .with_description("Total keepalive responses sent")
            .build();

        Self {
            keepalive_responses_counter,
        }
    }
}

impl KeepaliveResponder {
    fn try_respond(
        &self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if !state.is_initialized() {
            return;
        }

        if let keepalive_proto::State::Server(cookie) = &state.keepalive {
            tracing::debug!("responding to keepalive");
            self.keepalive_responses_counter.add(1, &[]);

            let msg = keepalive_proto::Message::ResponseKeepAlive(*cookie);
            send_to_peer(pid, state, AnyMessage::KeepAlive(msg), outbound);
        }
    }
}

impl ResponderPeerVisitor for KeepaliveResponder {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        self.try_respond(pid, state, outbound);
    }
}
