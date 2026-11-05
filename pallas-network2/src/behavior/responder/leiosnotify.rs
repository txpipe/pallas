use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId, behavior::AnyMessage,
    protocol::leiosnotify as notify_proto,
};

use super::{ResponderBehavior, ResponderEvent, ResponderPeerVisitor, ResponderState};

/// Responder sub-behavior that surfaces leios-notify `RequestNext` from peers as
/// [`ResponderEvent::EbNotificationRequested`], so the application can answer
/// with an announcement or offer (via the `Provide*` responder commands), and
/// that answers a peer's `Quit` with `Done`.
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

        match state.leios_notify {
            notify_proto::State::Busy => {
                tracing::debug!("leios notification requested");
                outbound.push_ready(BehaviorOutput::ExternalEvent(
                    ResponderEvent::EbNotificationRequested(pid.clone()),
                ));
            }
            notify_proto::State::Quit => {
                tracing::debug!("closing leios notify");
                outbound.push_ready(InterfaceCommand::Send(
                    pid.clone(),
                    AnyMessage::LeiosNotify(notify_proto::Message::Done),
                ));
            }
            _ => {}
        }
    }
}
