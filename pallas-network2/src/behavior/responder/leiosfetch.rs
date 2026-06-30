use crate::{BehaviorOutput, OutboundQueue, PeerId, protocol::leiosfetch as fetch_proto};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

/// Responder sub-behavior that surfaces leios-fetch requests from peers as
/// events, so the application can answer with the corresponding `Provide*`
/// responder commands.
#[derive(Default)]
pub struct LeiosFetchResponder;

impl ResponderPeerVisitor for LeiosFetchResponder {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if !state.is_initialized() {
            return;
        }

        let event = match &state.leios_fetch {
            fetch_proto::State::AwaitingBlock(point) => {
                Some(ResponderEvent::EbRequested(pid.clone(), point.clone()))
            }
            fetch_proto::State::AwaitingBlockTxs(point, bitmaps) => Some(
                ResponderEvent::EbTxsRequested(pid.clone(), point.clone(), bitmaps.clone()),
            ),
            // Idle/Done carry no inbound request to surface.
            _ => None,
        };

        if let Some(event) = event {
            tracing::debug!("leios fetch requested");
            outbound.push_ready(BehaviorOutput::ExternalEvent(event));
        }
    }
}
