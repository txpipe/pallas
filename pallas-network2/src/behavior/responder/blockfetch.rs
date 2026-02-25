use crate::{
    BehaviorOutput, OutboundQueue, PeerId,
    protocol::blockfetch as blockfetch_proto,
};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

#[derive(Default)]
pub struct BlockFetchResponder;

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
