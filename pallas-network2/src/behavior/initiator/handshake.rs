use std::time::Duration;

// tokio's clock when tokio is compiled in, so tests can drive the timeout with
// tokio::time::pause/advance; std's otherwise.
#[cfg(not(feature = "tokio"))]
pub(crate) use std::time::Instant;
#[cfg(feature = "tokio")]
pub(crate) use tokio::time::Instant;

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{AnyMessage, ConnectionState},
    protocol::MAINNET_MAGIC,
};

use super::{InitiatorBehavior, InitiatorEvent, InitiatorState, PeerVisitor};

/// How long [`HandshakeBehavior::new`] lets a connected peer take to complete
/// the handshake, with a safety margin over what the Haskell node allows for
/// the peer's reply to a `Propose` (`shortWait` in ouroboros-network's
/// `timeLimitsHandshake`).
pub const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// Configuration for the handshake sub-behavior.
pub struct Config {
    /// The N2N version table to propose during handshakes.
    pub supported_version: crate::protocol::handshake::n2n::VersionTable,
}

/// Sub-behavior that performs the handshake mini-protocol on new connections.
///
/// A peer that is still not initialized once the handshake timeout has
/// elapsed since it connected is treated as an errored connection: its error
/// count goes up and it is disconnected. The check runs on housekeeping, so the
/// peer is dropped on the first housekeeping after the timeout.
pub struct HandshakeBehavior {
    config: Config,
    timeout: Duration,
}

impl Default for HandshakeBehavior {
    fn default() -> Self {
        Self::new(Config {
            supported_version: crate::protocol::handshake::n2n::VersionTable {
                values: vec![(
                    13,
                    crate::protocol::handshake::n2n::VersionData {
                        network_magic: MAINNET_MAGIC,
                        initiator_only_diffusion_mode: false,
                        peer_sharing: Some(1),
                        //peer_sharing: Some(0),
                        query: Some(false),
                    },
                )]
                .into_iter()
                .collect(),
            },
        })
    }
}

impl HandshakeBehavior {
    /// Creates a new handshake behavior with the given configuration and
    /// [`DEFAULT_HANDSHAKE_TIMEOUT`].
    pub fn new(config: Config) -> Self {
        Self {
            config,
            timeout: DEFAULT_HANDSHAKE_TIMEOUT,
        }
    }

    /// Sets how long a connected peer may take to complete the handshake.
    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self { timeout, ..self }
    }

    fn drop_if_handshake_expired(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        if !matches!(state.connection, ConnectionState::Connected) {
            return;
        }

        let Some(deadline) = state.handshake_deadline else {
            return;
        };

        if Instant::now() < deadline {
            return;
        }

        tracing::warn!(%pid, timeout = ?self.timeout, "handshake timed out");

        state.handshake_deadline = None;
        state.connection = ConnectionState::Errored;
        state.error_count += 1;

        // TODO: once `InitiatorEvent::PeerDisconnected` lands (txpipe/pallas#842),
        // report this disconnect with a dedicated `DisconnectReason` (e.g.
        // `HandshakeTimeout`), so the consumer can tell it from a plain close.
        outbound.push_ready(InterfaceCommand::Disconnect(pid.clone()));
    }

    fn propose_handshake(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        assert!(matches!(
            state.handshake,
            crate::protocol::handshake::State::Propose
        ));

        let msg =
            crate::protocol::handshake::Message::Propose(self.config.supported_version.clone());

        let out = BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::Handshake(msg),
        ));

        outbound.push_ready(out);
    }

    fn check_confirmation(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        let crate::protocol::handshake::State::Done(
            crate::protocol::handshake::DoneState::Accepted(num, data),
        ) = &state.handshake
        else {
            return;
        };

        state.connection = crate::behavior::ConnectionState::Initialized;

        let out = BehaviorOutput::ExternalEvent(InitiatorEvent::PeerInitialized(
            pid.clone(),
            (*num, data.clone()),
        ));

        outbound.push_ready(out);
    }
}

fn needs_handshake(peer: &InitiatorState) -> bool {
    matches!(peer.connection, crate::behavior::ConnectionState::Connected)
}

impl PeerVisitor for HandshakeBehavior {
    fn visit_connected(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        state.handshake_deadline = Some(Instant::now() + self.timeout);
        self.propose_handshake(pid, state, outbound);
    }

    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.drop_if_handshake_expired(pid, state, outbound);
    }

    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if needs_handshake(state) {
            self.check_confirmation(pid, state, outbound);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutboundQueue;
    use crate::behavior::ConnectionState;
    use crate::protocol::handshake;

    fn drain_outputs(
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) -> Vec<BehaviorOutput<InitiatorBehavior>> {
        outbound.drain_ready()
    }

    #[test]
    fn propose_sent_on_connect() {
        let mut hs = HandshakeBehavior::default();
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        hs.visit_connected(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        assert_eq!(outputs.len(), 1);

        let is_propose = outputs.iter().any(|o| {
            matches!(
                o,
                BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    _,
                    AnyMessage::Handshake(handshake::Message::Propose(_))
                ))
            )
        });
        assert!(is_propose, "should send a Propose message on connect");
    }

    #[test]
    fn accepted_handshake_sets_initialized() {
        let mut hs = HandshakeBehavior::default();
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        // Put state into Done(Accepted) as if the handshake completed
        let version_data = crate::protocol::handshake::n2n::VersionData::new(
            MAINNET_MAGIC,
            false,
            Some(1),
            Some(false),
        );
        state.handshake = handshake::State::Done(handshake::DoneState::Accepted(13, version_data));
        state.connection = ConnectionState::Connected;

        hs.visit_inbound_msg(&pid, &mut state, &mut outbound);

        assert_eq!(
            state.connection,
            ConnectionState::Initialized,
            "connection should be set to Initialized after accepted handshake"
        );

        let outputs = drain_outputs(&mut outbound);
        let has_init_event = outputs.iter().any(|o| {
            matches!(
                o,
                BehaviorOutput::ExternalEvent(InitiatorEvent::PeerInitialized(..))
            )
        });
        assert!(has_init_event, "should emit PeerInitialized event");
    }

    #[cfg(feature = "tokio")]
    fn is_disconnect(output: &BehaviorOutput<InitiatorBehavior>, pid: &PeerId) -> bool {
        matches!(
            output,
            BehaviorOutput::InterfaceCommand(InterfaceCommand::Disconnect(p)) if p == pid
        )
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(start_paused = true)]
    async fn unanswered_handshake_disconnects_after_the_timeout() {
        let mut hs = HandshakeBehavior::default().with_timeout(Duration::from_secs(10));
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.connection = ConnectionState::Connected;
        hs.visit_connected(&pid, &mut state, &mut outbound);
        drain_outputs(&mut outbound);

        tokio::time::advance(Duration::from_secs(9)).await;
        hs.visit_housekeeping(&pid, &mut state, &mut outbound);
        assert!(
            drain_outputs(&mut outbound).is_empty(),
            "should wait until the timeout"
        );
        assert_eq!(state.connection, ConnectionState::Connected);

        tokio::time::advance(Duration::from_secs(1)).await;
        hs.visit_housekeeping(&pid, &mut state, &mut outbound);
        let outputs = drain_outputs(&mut outbound);
        assert!(
            outputs.iter().any(|o| is_disconnect(o, &pid)),
            "should disconnect the peer once the timeout has elapsed"
        );
        assert_eq!(state.connection, ConnectionState::Errored);
        assert_eq!(state.error_count, 1);
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(start_paused = true)]
    async fn initialized_peer_is_kept_after_the_timeout() {
        let mut hs = HandshakeBehavior::default().with_timeout(Duration::from_secs(10));
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.connection = ConnectionState::Connected;
        hs.visit_connected(&pid, &mut state, &mut outbound);
        drain_outputs(&mut outbound);
        state.connection = ConnectionState::Initialized;

        tokio::time::advance(Duration::from_secs(60)).await;
        hs.visit_housekeeping(&pid, &mut state, &mut outbound);

        assert!(drain_outputs(&mut outbound).is_empty());
        assert_eq!(state.connection, ConnectionState::Initialized);
        assert_eq!(state.error_count, 0);
    }

    #[test]
    fn non_connected_state_skips_confirmation() {
        let mut hs = HandshakeBehavior::default();
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        // State is Initialized (not Connected), so needs_handshake returns false
        state.connection = ConnectionState::Initialized;

        hs.visit_inbound_msg(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        assert!(
            outputs.is_empty(),
            "should not produce output when not in Connected state"
        );
    }
}
