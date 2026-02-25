use crate::{
    BehaviorOutput, OutboundQueue, PeerId,
    protocol::chainsync as chainsync_proto,
};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

#[derive(Default)]
pub struct ChainSyncResponder;

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
