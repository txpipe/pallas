use std::time::Duration;

use crate::{
    InterfaceCommand, OutboundQueue, PeerId,
    behavior::{AnyMessage, ConnectionState},
    protocol::keepalive::State,
};

use super::{InitiatorBehavior, InitiatorState, PeerVisitor, handshake::Instant};

/// How long [`KeepaliveBehavior::default`] lets a peer take to answer a
/// keepalive, with a safety margin over what the Haskell node intends for that
/// reply (`timeLimitsKeepAlive` in ouroboros-network, `TODO: #2505`).
pub const DEFAULT_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(30);

/// Sub-behavior that sends periodic keepalive messages to maintain connections.
///
/// A peer that has not answered a keepalive once the timeout has elapsed
/// since it was sent is treated as an errored connection: its error count goes
/// up and it is disconnected. The check runs on housekeeping, so the peer is
/// dropped on the first housekeeping after the timeout.
pub struct KeepaliveBehavior {
    token: u16,
    timeout: Duration,
}

impl Default for KeepaliveBehavior {
    fn default() -> Self {
        Self {
            token: u16::MAX,
            timeout: DEFAULT_KEEPALIVE_TIMEOUT,
        }
    }
}

impl KeepaliveBehavior {
    /// Sets how long a peer may take to answer a keepalive.
    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self { timeout, ..self }
    }

    /// Sends a keepalive message to the peer if the protocol state allows it.
    pub fn send_keepalive(
        &mut self,
        pid: &PeerId,
        peer: &InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if !peer.is_initialized() {
            return;
        }

        if matches!(peer.keepalive, crate::protocol::keepalive::State::Client(_)) {
            let msg = crate::protocol::keepalive::Message::KeepAlive(self.token);

            let out = InterfaceCommand::Send(pid.clone(), AnyMessage::KeepAlive(msg));

            outbound.push_ready(out);
        }
    }

    fn drop_if_keepalive_expired(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if !state.is_initialized() || !matches!(state.keepalive, State::Server(_)) {
            return;
        }

        let Some(deadline) = state.keepalive_deadline else {
            return;
        };

        if Instant::now() < deadline {
            return;
        }

        tracing::warn!(%pid, timeout = ?self.timeout, "keepalive timed out");

        state.keepalive_deadline = None;
        state.connection = ConnectionState::Errored;
        state.error_count += 1;

        // TODO: once `InitiatorEvent::PeerDisconnected` lands (txpipe/pallas#842),
        // report this disconnect with a dedicated `DisconnectReason` (e.g.
        // `KeepaliveTimeout`), so the consumer can tell it from a plain close.
        outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
    }
}

impl PeerVisitor for KeepaliveBehavior {
    fn visit_outbound_msg(
        &mut self,
        _pid: &PeerId,
        state: &mut InitiatorState,
        _outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        // The keepalive state turns to `Server` once our keepalive is sent: the
        // peer now has to answer it.
        if matches!(state.keepalive, State::Server(_)) && state.keepalive_deadline.is_none() {
            state.keepalive_deadline = Some(Instant::now() + self.timeout);
        }
    }

    fn visit_inbound_msg(
        &mut self,
        _pid: &PeerId,
        state: &mut InitiatorState,
        _outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if !matches!(state.keepalive, State::Server(_)) {
            state.keepalive_deadline = None;
        }
    }

    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.drop_if_keepalive_expired(pid, state, outbound);

        if state.is_initialized() {
            let msg = crate::protocol::keepalive::Message::KeepAlive(self.token);
            state.try_send_request(pid, AnyMessage::KeepAlive(msg), outbound);
        }
    }
}

#[cfg(all(test, feature = "tokio"))]
mod tests {
    use super::*;
    use crate::BehaviorOutput;

    fn is_disconnect(output: &BehaviorOutput<InitiatorBehavior>, pid: &PeerId) -> bool {
        matches!(
            output,
            BehaviorOutput::InterfaceCommand(InterfaceCommand::Disconnect(p)) if p == pid
        )
    }

    /// An initialized peer whose keepalive has just been sent.
    fn awaiting_keepalive_reply(
        ka: &mut KeepaliveBehavior,
        pid: &PeerId,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) -> InitiatorState {
        let mut state = InitiatorState::new();
        state.connection = ConnectionState::Initialized;
        state.keepalive = State::Server(u16::MAX);
        ka.visit_outbound_msg(pid, &mut state, outbound);
        state
    }

    #[tokio::test(start_paused = true)]
    async fn unanswered_keepalive_disconnects_after_the_timeout() {
        let mut ka = KeepaliveBehavior::default().with_timeout(Duration::from_secs(10));
        let pid = PeerId::test(1);
        let mut outbound = OutboundQueue::new();
        let mut state = awaiting_keepalive_reply(&mut ka, &pid, &mut outbound);

        tokio::time::advance(Duration::from_secs(9)).await;
        ka.visit_housekeeping(&pid, &mut state, &mut outbound);
        assert!(
            !outbound
                .drain_ready()
                .iter()
                .any(|o| is_disconnect(o, &pid)),
            "should wait until the timeout"
        );
        assert_eq!(state.connection, ConnectionState::Initialized);

        tokio::time::advance(Duration::from_secs(1)).await;
        ka.visit_housekeeping(&pid, &mut state, &mut outbound);
        assert!(
            outbound
                .drain_ready()
                .iter()
                .any(|o| is_disconnect(o, &pid)),
            "should disconnect the peer once the timeout has elapsed"
        );
        assert_eq!(state.connection, ConnectionState::Errored);
        assert_eq!(state.error_count, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn answered_keepalive_keeps_the_peer() {
        let mut ka = KeepaliveBehavior::default().with_timeout(Duration::from_secs(10));
        let pid = PeerId::test(1);
        let mut outbound = OutboundQueue::new();
        let mut state = awaiting_keepalive_reply(&mut ka, &pid, &mut outbound);

        state.keepalive =
            State::Client(crate::protocol::keepalive::ClientState::Response(u16::MAX));
        ka.visit_inbound_msg(&pid, &mut state, &mut outbound);

        tokio::time::advance(Duration::from_secs(60)).await;
        ka.visit_housekeeping(&pid, &mut state, &mut outbound);

        assert!(
            !outbound
                .drain_ready()
                .iter()
                .any(|o| is_disconnect(o, &pid))
        );
        assert_eq!(state.connection, ConnectionState::Initialized);
        assert_eq!(state.error_count, 0);
    }
}
