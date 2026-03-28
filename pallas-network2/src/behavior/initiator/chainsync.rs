use crate::protocol::{Point, chainsync as chainsync_proto};

use crate::{
    BehaviorOutput, InterfaceCommand, OutboundQueue, PeerId,
    behavior::{AnyMessage, ConnectionState},
};

use super::{InitiatorBehavior, InitiatorEvent, InitiatorState, PeerVisitor, PromotionTag};

pub type ChainSyncConfig = ();

type Intersection = Vec<Point>;

pub struct ChainSyncBehavior {
    //config: ChainSyncConfig,
    intersection: Option<Intersection>,
}

impl Default for ChainSyncBehavior {
    fn default() -> Self {
        Self::new(())
    }
}

impl ChainSyncBehavior {
    pub fn new(_config: ChainSyncConfig) -> Self {
        Self { intersection: None }
    }

    pub fn start(&mut self, intersection: Intersection) {
        self.intersection = Some(intersection);
    }

    pub fn request_intersection(
        &self,
        pid: &PeerId,
        intersection: &Intersection,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        tracing::debug!("requesting intersection");

        let msg = chainsync_proto::Message::FindIntersect(intersection.clone());

        outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::ChainSync(msg),
        )));
    }

    pub fn request_next(
        &self,
        pid: &PeerId,
        _state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        tracing::debug!("requesting next header");

        let out = chainsync_proto::Message::RequestNext;
        outbound.push_ready(BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
            pid.clone(),
            AnyMessage::ChainSync(out),
        )));
    }

    pub fn drain_data(
        &self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<super::InitiatorBehavior>,
    ) {
        let Some(data) = state.chainsync.drain() else {
            return;
        };

        match data {
            chainsync_proto::Data::Content(h, tip) => {
                let out = InitiatorEvent::BlockHeaderReceived(pid.clone(), h.clone(), tip.clone());
                outbound.push_ready(BehaviorOutput::ExternalEvent(out));
            }
            chainsync_proto::Data::Rollback(point, tip) => {
                let out = InitiatorEvent::RollbackReceived(pid.clone(), point.clone(), tip.clone());
                outbound.push_ready(BehaviorOutput::ExternalEvent(out));
            }
            chainsync_proto::Data::Intersection(point, tip) => {
                let out =
                    InitiatorEvent::IntersectionFound(pid.clone(), point.clone(), tip.clone());
                outbound.push_ready(BehaviorOutput::ExternalEvent(out));
            }
            chainsync_proto::Data::NoIntersection(..) => {
                tracing::error!("no intersection found");
                state.violation = true;
            }
            _ => (),
        }
    }
}

fn peer_is_syncing(state: &InitiatorState) -> bool {
    !state.chainsync.is_new()
}

fn peer_should_sync(state: &InitiatorState) -> bool {
    matches!(state.connection, ConnectionState::Initialized) && state.promotion == PromotionTag::Hot
}

impl PeerVisitor for ChainSyncBehavior {
    fn visit_inbound_msg(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if !peer_is_syncing(state) {
            tracing::trace!("peer is not syncing, skipping drain data");
            return;
        }

        self.drain_data(pid, state, outbound);
    }

    fn visit_tagged(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        if !peer_is_syncing(state) {
            tracing::trace!("peer is not syncing, skipping tagged");
            return;
        }

        if !state.chainsync.is_idle() {
            tracing::trace!("chainsync is not idle, skipping request next");
            return;
        }

        if state.continue_sync {
            tracing::debug!("peer wants to continue sync");
            self.request_next(pid, state, outbound);
        }
    }

    fn visit_housekeeping(
        &mut self,
        pid: &PeerId,
        state: &mut InitiatorState,
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) {
        let Some(intersection) = &self.intersection else {
            tracing::trace!("no intersection requested");
            return;
        };

        if !peer_should_sync(state) {
            tracing::trace!("peer is not suitable for sync");
            return;
        }

        if peer_is_syncing(state) {
            tracing::trace!("peer is already syncing");
            return;
        }

        tracing::trace!("peer needs to sync");

        self.request_intersection(pid, intersection, outbound);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::chainsync::{Data, HeaderContent, State as CsState, Tip};
    use crate::OutboundQueue;
    use futures::StreamExt;

    fn make_peer() -> PeerId {
        PeerId {
            host: "10.0.0.1".to_string(),
            port: 3001,
        }
    }

    fn drain_outputs(
        outbound: &mut OutboundQueue<InitiatorBehavior>,
    ) -> Vec<BehaviorOutput<InitiatorBehavior>> {
        let mut outputs = Vec::new();
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        loop {
            match outbound.futures.poll_next_unpin(&mut cx) {
                std::task::Poll::Ready(Some(output)) => outputs.push(output),
                _ => break,
            }
        }

        outputs
    }

    fn mock_tip() -> Tip {
        Tip(Point::new(100, vec![0xAA; 32]), 100)
    }

    fn mock_header() -> HeaderContent {
        HeaderContent {
            variant: 1,
            byron_prefix: None,
            cbor: vec![0xBE; 32],
        }
    }

    #[test]
    fn drain_content_emits_header_event() {
        let cs = ChainSyncBehavior::new(());
        let pid = make_peer();
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.chainsync = CsState::Idle(Data::Content(mock_header(), mock_tip()));

        cs.drain_data(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let has_event = outputs.iter().any(|o| {
            matches!(o, BehaviorOutput::ExternalEvent(InitiatorEvent::BlockHeaderReceived(..)))
        });
        assert!(has_event);
    }

    #[test]
    fn drain_rollback_emits_rollback_event() {
        let cs = ChainSyncBehavior::new(());
        let pid = make_peer();
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.chainsync = CsState::Idle(Data::Rollback(Point::Origin, mock_tip()));

        cs.drain_data(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let has_event = outputs.iter().any(|o| {
            matches!(o, BehaviorOutput::ExternalEvent(InitiatorEvent::RollbackReceived(..)))
        });
        assert!(has_event);
    }

    #[test]
    fn drain_intersection_emits_found_event() {
        let cs = ChainSyncBehavior::new(());
        let pid = make_peer();
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.chainsync = CsState::Idle(Data::Intersection(Point::Origin, mock_tip()));

        cs.drain_data(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let has_event = outputs.iter().any(|o| {
            matches!(o, BehaviorOutput::ExternalEvent(InitiatorEvent::IntersectionFound(..)))
        });
        assert!(has_event);
    }

    #[test]
    fn no_intersection_sets_violation() {
        let cs = ChainSyncBehavior::new(());
        let pid = make_peer();
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.chainsync = CsState::Idle(Data::NoIntersection(mock_tip()));

        cs.drain_data(&pid, &mut state, &mut outbound);

        assert!(state.violation, "NoIntersection should set violation flag");
    }

    #[test]
    fn housekeeping_starts_sync_for_hot_initialized_peer() {
        let mut cs = ChainSyncBehavior::new(());
        cs.start(vec![Point::Origin]);

        let pid = make_peer();
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.connection = ConnectionState::Initialized;
        state.promotion = PromotionTag::Hot;
        // chainsync default is Idle(New) → is_new() returns true → not syncing

        cs.visit_housekeeping(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let has_find = outputs.iter().any(|o| {
            matches!(
                o,
                BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    _,
                    AnyMessage::ChainSync(chainsync_proto::Message::FindIntersect(_))
                ))
            )
        });
        assert!(has_find, "should send FindIntersect for Hot+Initialized peer");
    }

    #[test]
    fn housekeeping_skips_non_hot_peer() {
        let mut cs = ChainSyncBehavior::new(());
        cs.start(vec![Point::Origin]);

        let pid = make_peer();
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        state.connection = ConnectionState::Initialized;
        state.promotion = PromotionTag::Warm; // Not Hot

        cs.visit_housekeeping(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        assert!(outputs.is_empty(), "should not start sync for Warm peer");
    }

    #[test]
    fn tagged_requests_next_when_continue_sync() {
        let mut cs = ChainSyncBehavior::new(());
        let pid = make_peer();
        let mut state = InitiatorState::new();
        let mut outbound = OutboundQueue::new();

        // Peer is syncing (not new) and idle with drained data
        state.chainsync = CsState::Idle(Data::Drained);
        state.continue_sync = true;

        cs.visit_tagged(&pid, &mut state, &mut outbound);

        let outputs = drain_outputs(&mut outbound);
        let has_next = outputs.iter().any(|o| {
            matches!(
                o,
                BehaviorOutput::InterfaceCommand(InterfaceCommand::Send(
                    _,
                    AnyMessage::ChainSync(chainsync_proto::Message::RequestNext)
                ))
            )
        });
        assert!(has_next, "should send RequestNext when continue_sync is set");
    }
}
