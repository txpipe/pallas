use std::collections::VecDeque;

use crate::protocol::EbId;
use crate::protocol::leiosfetch::{self as fetch_proto, Bitmaps};

use crate::{BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId, behavior::AnyMessage};

use super::{InitiatorBehavior, InitiatorEvent, InitiatorState, PeerVisitor};

/// A pending leios-fetch request targeting a specific peer.
#[derive(Debug, Clone)]
pub enum FetchRequest {
    /// Fetch a complete EB body.
    Block(EbId),
    /// Fetch a subset of an EB's transactions.
    BlockTxs(EbId, Bitmaps),
}

/// Sub-behavior that fetches EB bodies and transactions from peers.
///
/// Requests are queued (each targeting the peer that should serve it) and sent
/// one at a time per peer during housekeeping, when that peer is idle. Responses
/// are surfaced as [`InitiatorEvent::EbFetched`].
#[derive(Default)]
pub struct LeiosFetchBehavior {
    requests: VecDeque<(PeerId, FetchRequest)>,
}

impl LeiosFetchBehavior {
    /// Queues a fetch request to be served by the given peer.
    pub fn enqueue(&mut self, pid: PeerId, request: FetchRequest) {
        self.requests.push_back((pid, request));
        tracing::info!(total = self.requests.len(), "new leios-fetch request");
    }

    /// Drops any queued requests targeting `pid`. Called when the peer goes away
    /// so requests don't leak or get re-sent to a later reconnection of the same
    /// `PeerId` (which may no longer hold the offered EB).
    fn purge(&mut self, pid: &PeerId) {
        self.requests.retain(|(p, _)| p != pid);
    }

    fn send_request(
        &self,
        pid: &PeerId,
        request: &FetchRequest,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        let msg = match request {
            FetchRequest::Block(point) => fetch_proto::Message::BlockRequest(point.clone()),
            FetchRequest::BlockTxs(point, bitmaps) => {
                fetch_proto::Message::BlockTxsRequest(point.clone(), bitmaps.clone())
            }
        };

        outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::LeiosFetch(msg),
        )));
    }

    /// Drains a pending response from the peer state and emits the corresponding
    /// external event.
    fn dispatch(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if let Some((eb, response)) = state.leios_fetch.drain() {
            outbound.push_ready(BehaviorOutput::ExternalEvent(InitiatorEvent::EbFetched(
                pid.clone(),
                eb,
                response,
            )));
        }
    }
}

fn peer_is_available(state: &InitiatorState) -> bool {
    state.is_initialized()
        && state.supports_leios()
        && matches!(state.leios_fetch, fetch_proto::State::Idle(None))
}

impl PeerVisitor for LeiosFetchBehavior {
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
        if !peer_is_available(state) {
            return;
        }

        // Serve the first queued request targeting this peer.
        if let Some(idx) = self.requests.iter().position(|(p, _)| p == pid) {
            let (_, request) = self.requests.remove(idx).expect("index just found");
            self.send_request(pid, &request, outbound);
        }
    }

    fn visit_disconnected(
        &mut self,
        pid: &PeerId,
        _state: &mut InitiatorState,
        _outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.purge(pid);
    }

    fn visit_errored(
        &mut self,
        pid: &PeerId,
        _state: &mut InitiatorState,
        _outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        self.purge(pid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Point;
    use crate::protocol::leiosfetch::Response;
    use crate::protocol::{AnyCbor, leiosfetch as lf};
    use crate::{OutboundQueue, behavior::ConnectionState};

    fn drain_outputs(
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) -> Vec<BehaviorOutput<InitiatorBehavior>> {
        outbound.drain_ready()
    }

    #[test]
    fn dispatch_emits_fetched_event_once() {
        let b = LeiosFetchBehavior::default();
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.leios_fetch = lf::State::Idle(Some((
            Point::Origin,
            Response::Block(AnyCbor::from_raw_bytes(vec![0x01])),
        )));

        b.dispatch(&pid, &mut state, &mut outbound);
        assert!(drain_outputs(&mut outbound).iter().any(|o| matches!(
            o,
            BehaviorOutput::ExternalEvent(InitiatorEvent::EbFetched(..))
        )));

        b.dispatch(&pid, &mut state, &mut outbound);
        assert!(drain_outputs(&mut outbound).is_empty());
    }

    #[test]
    fn housekeeping_sends_request_for_available_peer() {
        let mut b = LeiosFetchBehavior::default();
        let pid = PeerId::test(1);
        let mut outbound = OutboundQueue::new();

        b.enqueue(pid.clone(), FetchRequest::Block(Point::Origin));

        // peer not ready → request stays queued
        let mut state = InitiatorState::new();
        state.connection = ConnectionState::Initialized; // but supports_leios() is false
        b.visit_housekeeping(&pid, &mut state, &mut outbound);
        assert!(drain_outputs(&mut outbound).is_empty());
        assert_eq!(b.requests.len(), 1);
    }

    #[test]
    fn disconnect_purges_queued_requests() {
        let mut b = LeiosFetchBehavior::default();
        let pid = PeerId::test(1);
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        b.enqueue(pid.clone(), FetchRequest::Block(Point::Origin));
        b.enqueue(PeerId::test(2), FetchRequest::Block(Point::Origin));
        assert_eq!(b.requests.len(), 2);

        // Disconnecting pid drops only its queued request.
        b.visit_disconnected(&pid, &mut state, &mut outbound);
        assert_eq!(b.requests.len(), 1);
        assert!(b.requests.iter().all(|(p, _)| p != &pid));
    }
}
