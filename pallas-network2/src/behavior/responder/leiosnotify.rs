use crate::{BehaviorOutput, OutboundQueue, PeerId, protocol::leiosnotify as notify_proto};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

/// Responder sub-behavior that surfaces leios-notify `RequestNext` from peers as
/// [`ResponderEvent::EbNotificationRequested`], so the application can answer
/// with an announcement or offer (via the `Provide*` responder commands).
#[derive(Default)]
pub struct LeiosNotifyResponder;

impl ResponderPeerVisitor for LeiosNotifyResponder {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut ResponderState,
        outbound: &mut OutboundQueue<ResponderBehavior>,
    ) {
        if !state.is_initialized() {
            return;
        }

        if matches!(state.leios_notify, notify_proto::State::Busy) {
            tracing::debug!("leios notification requested");
            outbound.push_ready(BehaviorOutput::ExternalEvent(
                ResponderEvent::EbNotificationRequested(pid.clone()),
            ));
        }
    }
}
