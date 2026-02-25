use crate::{
    BehaviorOutput, OutboundQueue, PeerId,
    protocol::peersharing as peersharing_proto,
};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

#[derive(Default)]
pub struct PeerSharingResponder;

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
