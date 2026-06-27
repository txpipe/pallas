use crate::protocol::leiosnotify as notify_proto;

use crate::{BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId, behavior::AnyMessage};

use super::{InitiatorBehavior, InitiatorEvent, InitiatorState, PeerVisitor};

/// Sub-behavior that drives the leios-notify pull loop and surfaces EB
/// announcements/offers received from peers.
///
/// `RequestNext` is issued during housekeeping whenever the peer is initialized,
/// negotiated a Leios-capable version, and the protocol is idle with nothing
/// pending — yielding a continuous notification loop paced by housekeeping.
#[derive(Default)]
pub struct LeiosNotifyBehavior;

impl LeiosNotifyBehavior {
    fn request_next(&self, pid: &PeerId, outbound: &mut OutboundQueue<InitiatorBehavior>) {
        tracing::debug!("requesting next leios notification");

        outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::LeiosNotify(notify_proto::Message::RequestNext),
        )));
    }

    /// Drains a pending notification from the peer state and emits the
    /// corresponding external event.
    fn dispatch(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if let Some(notification) = state.leios_notify.drain() {
            outbound.push_ready(BehaviorOutput::ExternalEvent(
                InitiatorEvent::EbNotification(pid.clone(), notification),
            ));
        }
    }
}

fn peer_ready(state: &InitiatorState) -> bool {
    state.is_initialized() && state.supports_leios()
}

impl PeerVisitor for LeiosNotifyBehavior {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.dispatch(pid, state, outbound);
    }

    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if !peer_ready(state) {
            return;
        }

        // Only request when idle with nothing pending; the Sent event will move
        // the protocol to Busy before the next housekeeping pass, avoiding
        // duplicate requests.
        if matches!(state.leios_notify, notify_proto::State::Idle(None)) {
            self.request_next(pid, outbound);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Point;
    use crate::protocol::leiosnotify::Notification;
    use crate::{OutboundQueue, behavior::ConnectionState};

    fn drain_outputs(
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) -> Vec<BehaviorOutput<InitiatorBehavior>> {
        outbound.drain_ready()
    }

    #[test]
    fn dispatch_emits_notification_event() {
        let b = LeiosNotifyBehavior;
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.leios_notify =
            notify_proto::State::Idle(Some(Notification::BlockOffer(Point::Origin, 10)));

        b.dispatch(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        assert!(outputs.iter().any(|o| matches!(
            o,
            BehaviorOutput::ExternalEvent(InitiatorEvent::EbNotification(..))
        )));
        // drained, so a second dispatch emits nothing
        b.dispatch(&pid, &mut state, &mut outbound);
        assert!(drain_outputs(&mut outbound).is_empty());
    }

    #[test]
    fn housekeeping_requests_next_only_when_ready_and_idle() {
        let mut b = LeiosNotifyBehavior;
        let pid = PeerId::test(1);
        let mut outbound = OutboundQueue::new();

        // not initialized → nothing
        let mut state = InitiatorState::new();
        b.visit_housekeeping(&pid, &mut state, &mut outbound);
        assert!(drain_outputs(&mut outbound).is_empty());

        // initialized but no Leios version → nothing
        state.connection = ConnectionState::Initialized;
        b.visit_housekeeping(&pid, &mut state, &mut outbound);
        assert!(drain_outputs(&mut outbound).is_empty());
    }
}
