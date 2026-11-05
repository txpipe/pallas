use crate::protocol::leiosnotify as notify_proto;

use crate::{BehaviorOutput, OutboundQueue, PeerId, behavior::AnyMessage};

use super::{InitiatorBehavior, InitiatorEvent, InitiatorState, PeerVisitor};

/// Sub-behavior that drives the leios-notify pull loop and surfaces EB
/// announcements/offers received from peers.
///
/// `RequestNext` is issued during housekeeping whenever the peer is initialized,
/// negotiated a Leios-capable version, and the protocol is idle with nothing
/// pending and no unsent `RequestNext`, yielding a continuous
/// notification loop paced by housekeeping.
#[derive(Default)]
pub struct LeiosNotifyBehavior;

impl LeiosNotifyBehavior {
    fn request_next(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        tracing::debug!("requesting next leios notification");

        let msg = AnyMessage::LeiosNotify(notify_proto::Message::RequestNext);
        state.try_send_request(pid, msg, outbound);
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

        if matches!(state.leios_notify, notify_proto::State::Idle(None)) {
            self.request_next(pid, state, outbound);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Point;
    use crate::protocol::leiosnotify::Notification;
    use crate::protocol::{MAINNET_MAGIC, handshake};
    use crate::{InterfaceCommand, OutboundQueue, behavior::ConnectionState};

    fn drain_outputs(
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) -> Vec<BehaviorOutput<InitiatorBehavior>> {
        outbound.drain_ready()
    }

    /// Puts the peer state into a handshake accepted at `version`, with the
    /// version data pallas proposes for it (shape depends on the version).
    fn accept_version(state: &mut InitiatorState, version: u64) {
        let data = handshake::n2n::VersionTable::v15_and_above(MAINNET_MAGIC)
            .values
            .get(&version)
            .cloned()
            .unwrap_or_else(|| panic!("pallas does not propose version {version}"));
        state.handshake = handshake::State::Done(handshake::DoneState::Accepted(version, data));
    }

    fn did_send_leios_notify(outputs: &[BehaviorOutput<InitiatorBehavior>]) -> bool {
        outputs.iter().any(|o| {
            matches!(
                o,
                BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    _,
                    AnyMessage::LeiosNotify(_)
                ))
            )
        })
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
    fn housekeeping_requests_nothing_before_handshake() {
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

    #[test]
    fn housekeeping_does_not_open_leios_notify_on_v15_peer() {
        // v15 is mainline's SRV support: the peer has no leios-notify mini-protocol.
        let mut b = LeiosNotifyBehavior;
        let pid = PeerId::test(1);
        let mut outbound = OutboundQueue::new();

        let mut state = InitiatorState::new();
        state.connection = ConnectionState::Initialized;
        accept_version(&mut state, 15);
        b.visit_housekeeping(&pid, &mut state, &mut outbound);
        assert!(!did_send_leios_notify(&drain_outputs(&mut outbound)));
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn housekeeping_sends_leios_notify_request_on_v16_peer() {
        let mut b = LeiosNotifyBehavior;
        let pid = PeerId::test(1);
        let mut outbound = OutboundQueue::new();

        let mut state = InitiatorState::new();
        state.connection = ConnectionState::Initialized;
        accept_version(&mut state, 16);
        b.visit_housekeeping(&pid, &mut state, &mut outbound);
        assert!(did_send_leios_notify(&drain_outputs(&mut outbound)));
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn housekeeping_sends_no_leios_notify_request_while_busy() {
        let mut b = LeiosNotifyBehavior;
        let pid = PeerId::test(1);
        let mut outbound = OutboundQueue::new();

        let mut state = InitiatorState::new();
        state.connection = ConnectionState::Initialized;
        accept_version(&mut state, 16);
        state.leios_notify = notify_proto::State::Busy;
        b.visit_housekeeping(&pid, &mut state, &mut outbound);
        assert!(!did_send_leios_notify(&drain_outputs(&mut outbound)));
    }
}
